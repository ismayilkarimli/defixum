mod browser;
mod macos;
mod rules;

use browser::{AxState, Fetch, MainRunner, Target};
use chrono::Local;
use rules::{Config, DayStats, Reason, Stats};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

const TICK: Duration = Duration::from_millis(900);
const STATS_KEEP_DAYS: usize = 120;

// ---------- logging ----------

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

fn log(msg: impl AsRef<str>) {
    let line = format!("{} {}\n", Local::now().format("%H:%M:%S"), msg.as_ref());
    eprint!("{line}");
    if let Some(p) = LOG_PATH.get() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

// ---------- state ----------

#[derive(Clone, Serialize)]
struct Intercept {
    rule_id: String,
    rule_name: String,
    site: String,
    reason: Reason,
    pause_seconds: u32,
    opens_today: u32,
    minutes_today: u64,
    daily_opens: Option<u32>,
    daily_minutes: Option<u32>,
    session_options: Vec<u32>,
    until: Option<String>,
    #[serde(skip)]
    shown_at: Instant,
    #[serde(skip)]
    target: Target,
}

struct Core {
    config: Config,
    stats: Stats,
    stats_dirty: bool,
    stats_saved: Instant,
    /// rule id → end of the "continue" window the user picked
    allowances: HashMap<String, Instant>,
    /// rule id → when its allowance ran out (to say "time's up" instead of a plain pause)
    expired: HashMap<String, Instant>,
    intercept: Option<Intercept>,
    /// after "go back", don't re-trigger for a moment while the tab navigates away
    cooldown: HashMap<String, Instant>,
    last_target: Option<Target>,
    /// bundle id → "ok" | "denied" | "error"
    browser_status: HashMap<String, &'static str>,
    data_dir: PathBuf,
}

type Shared = Arc<Mutex<Core>>;

struct Runner(Arc<MainRunner>);

impl Core {
    fn save_config(&self) {
        write_json(&self.data_dir.join("config.json"), &self.config);
    }

    fn save_stats(&mut self) {
        self.stats.prune(STATS_KEEP_DAYS);
        write_json(&self.data_dir.join("stats.json"), &self.stats);
        self.stats_dirty = false;
        self.stats_saved = Instant::now();
    }

    fn calm_url(&self) -> String {
        let path = self.data_dir.join("calm.html");
        format!("file://{}", path.to_string_lossy().replace(' ', "%20"))
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    let tmp = path.with_extension("tmp");
    if let Ok(text) = serde_json::to_string_pretty(value) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log(format!("could not parse {}: {e}", path.display()));
            let _ = std::fs::copy(path, path.with_extension("broken.json"));
            None
        }
    }
}

// ---------- overlay window ----------

fn overlay_show(app: &AppHandle, payload: &Intercept) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        // Size first: AppKit keeps the bottom-left corner fixed on resize, so resizing after
        // positioning would push the top of the window off screen.
        let _ = w.set_size(*m.size());
        let _ = w.set_position(*m.position());
    }
    let _ = app.emit_to("overlay", "intercept", payload);
    let _ = w.show();
    let _ = w.set_focus();
    // Without this the web view only gets keystrokes (Esc) after a click.
    let _ = w.with_webview(|wv| {
        use objc2_app_kit::{NSView, NSWindow};
        let view: &NSView = unsafe { &*(wv.inner() as *const NSView) };
        let win: &NSWindow = unsafe { &*(wv.ns_window() as *const NSWindow) };
        win.makeKeyAndOrderFront(None);
        win.makeFirstResponder(Some(view));
    });
}

fn overlay_hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = app.emit_to("overlay", "dismiss", ());
        let _ = w.hide();
    }
}

/// Float the overlay above everything, including other apps' full-screen spaces.
fn raise_overlay(w: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    if let Ok(ptr) = w.ns_window() {
        let win: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
        win.setLevel(101); // NSPopUpMenuWindowLevel
        win.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
    }
}

fn open_dashboard(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = app.emit_to("main", "refresh", ());
    }
}

// ---------- monitor loop ----------

enum UiAction {
    None,
    Show(Intercept),
    Hide,
}

fn monitor(app: AppHandle, core: Shared, runner: Arc<MainRunner>) {
    let own_pid = std::process::id() as i32;
    let mut ax = AxState::default();
    let mut last = Instant::now();
    loop {
        std::thread::sleep(TICK);
        let dt = last.elapsed().min(Duration::from_secs(3));
        last = Instant::now();

        let overlay_up = core.lock().unwrap().intercept.is_some();
        let front = macos::frontmost();
        // (target, is the browser itself frontmost?)
        let target = match front {
            Some(f) if f.pid == own_pid => {
                if overlay_up {
                    core.lock().unwrap().last_target.clone().map(|t| (t, false))
                } else {
                    None
                }
            }
            Some(f) => browser::lookup(&f.bundle)
                .map(|k| (Target { bundle: f.bundle.clone(), pid: f.pid, kind: k.kind }, true)),
            None => None,
        };

        let Some((target, focused)) = target else {
            if overlay_up {
                core.lock().unwrap().intercept = None;
                overlay_hide(&app);
            }
            maybe_save_stats(&core);
            continue;
        };

        let fetched = browser::fetch(&target, &mut ax, runner.as_ref());
        let url = match &fetched {
            Fetch::Url(u) => Some(u.clone()),
            Fetch::Nothing => None,
            Fetch::Denied | Fetch::Unavailable => {
                let status = if fetched == Fetch::Denied { "denied" } else { "error" };
                core.lock().unwrap().browser_status.insert(target.bundle.clone(), status);
                continue;
            }
        };

        let action = {
            let mut c = core.lock().unwrap();
            c.browser_status.insert(target.bundle.clone(), "ok");
            c.last_target = Some(target.clone());
            decide(&mut c, &target, focused, url.as_deref(), dt)
        };
        match action {
            UiAction::Show(i) => {
                log(format!("intercept {} ({:?}) on {}", i.site, i.reason, i.target.bundle));
                overlay_show(&app, &i)
            }
            UiAction::Hide => overlay_hide(&app),
            UiAction::None => {}
        }
        maybe_save_stats(&core);
    }
}

fn maybe_save_stats(core: &Shared) {
    let mut c = core.lock().unwrap();
    if c.stats_dirty && c.stats_saved.elapsed() > Duration::from_secs(20) {
        c.save_stats();
    }
}

fn decide(c: &mut Core, target: &Target, focused: bool, url: Option<&str>, dt: Duration) -> UiAction {
    let now = Instant::now();
    let lapsed: Vec<String> = c.allowances.iter().filter(|(_, &t)| t <= now).map(|(k, _)| k.clone()).collect();
    for id in lapsed {
        c.allowances.remove(&id);
        c.expired.insert(id, now);
    }
    c.expired.retain(|_, t| t.elapsed() < Duration::from_secs(60));
    c.cooldown.retain(|_, t| *t > now);

    let local = Local::now();
    let verdict = url.and_then(|u| {
        rules::evaluate(&c.config, &c.stats, &local, u, &|id| c.allowances.contains_key(id))
    });
    let Some(v) = verdict else {
        return if c.intercept.take().is_some() { UiAction::Hide } else { UiAction::None };
    };
    let rule = c.config.rules[v.rule_index].clone();
    let today = rules::date_key(&local);

    let Some(mut reason) = v.reason else {
        // Inside a granted window: count the time while the user is actually there.
        if focused && macos::idle_seconds() < c.config.settings.idle_seconds as f64 {
            c.stats.day_mut(&today, &rule.id).seconds += dt.as_secs_f64().round() as u64;
            c.stats_dirty = true;
        }
        return if c.intercept.take().is_some() { UiAction::Hide } else { UiAction::None };
    };

    if c.intercept.as_ref().is_some_and(|i| i.rule_id == rule.id) || c.cooldown.contains_key(&rule.id) {
        return UiAction::None;
    }
    if reason == Reason::Pause && c.expired.contains_key(&rule.id) {
        reason = Reason::Expired;
    }
    c.allowances.remove(&rule.id);
    let day: DayStats = {
        let d = c.stats.day_mut(&today, &rule.id);
        if reason.can_continue() {
            d.intercepts += 1;
        }
        *d
    };
    c.stats_dirty = true;
    let until = match reason {
        Reason::Blocked if rule.schedule.enabled => Some(rule.schedule.end.clone()),
        Reason::OpenLimit | Reason::TimeLimit => Some("tomorrow".into()),
        _ => None,
    };
    let mut options: Vec<u32> = c.config.settings.session_options.iter().copied().filter(|&m| m > 0).collect();
    options.sort_unstable();
    options.dedup();
    options.truncate(4);
    let i = Intercept {
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        site: rules::host_of(url.unwrap_or_default()),
        reason,
        pause_seconds: rule.pause_seconds.clamp(1, 600),
        opens_today: day.opens,
        minutes_today: day.seconds / 60,
        daily_opens: rule.daily_opens,
        daily_minutes: rule.daily_minutes,
        session_options: options,
        until,
        shown_at: now,
        target: target.clone(),
    };
    c.intercept = Some(i.clone());
    UiAction::Show(i)
}

// ---------- commands: overlay ----------

#[tauri::command]
async fn overlay_state(core: State<'_, Shared>) -> Result<Option<Intercept>, String> {
    Ok(core.lock().unwrap().intercept.clone())
}

#[tauri::command]
async fn overlay_continue(app: AppHandle, core: State<'_, Shared>, minutes: u32) -> Result<(), String> {
    let pid = {
        let mut c = core.lock().unwrap();
        let i = c.intercept.as_ref().ok_or("no pause in progress")?;
        if !i.reason.can_continue() {
            return Err("this site is blocked right now".into());
        }
        if i.shown_at.elapsed().as_secs_f64() + 0.5 < i.pause_seconds as f64 {
            return Err("the pause isn't over yet".into());
        }
        let i = c.intercept.take().unwrap();
        let minutes = minutes.clamp(1, 240);
        c.allowances.insert(i.rule_id.clone(), Instant::now() + Duration::from_secs(minutes as u64 * 60));
        c.expired.remove(&i.rule_id);
        c.stats.day_mut(&rules::date_key(&Local::now()), &i.rule_id).opens += 1;
        c.save_stats();
        log(format!("continue {} for {minutes} min", i.site));
        i.target.pid
    };
    overlay_hide(&app);
    macos::activate(pid);
    Ok(())
}

#[tauri::command]
async fn overlay_back(app: AppHandle, core: State<'_, Shared>, runner: State<'_, Runner>) -> Result<(), String> {
    let (target, calm) = {
        let mut c = core.lock().unwrap();
        let i = c.intercept.take().ok_or("no pause in progress")?;
        if i.reason.can_continue() {
            c.stats.day_mut(&rules::date_key(&Local::now()), &i.rule_id).skips += 1;
            c.save_stats();
        }
        c.expired.remove(&i.rule_id);
        c.cooldown.insert(i.rule_id.clone(), Instant::now() + Duration::from_secs(3));
        log(format!("back from {}", i.site));
        (i.target, format!("{}#{}", c.calm_url(), i.site))
    };
    overlay_hide(&app);
    macos::activate(target.pid);
    let runner = runner.0.clone();
    let ok = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(250));
        browser::navigate(&target, &calm, runner.as_ref())
    })
    .await
    .unwrap_or(false);
    if !ok {
        log("could not navigate the tab away");
    }
    Ok(())
}

// ---------- commands: dashboard ----------

#[derive(Serialize)]
struct BrowserInfo {
    bundle: &'static str,
    name: &'static str,
    kind: browser::Kind,
    running: bool,
    status: Option<&'static str>,
}

#[derive(Serialize)]
struct DashState {
    config: Config,
    today: String,
    stats: BTreeMap<String, HashMap<String, DayStats>>,
    accessibility: bool,
    browsers: Vec<BrowserInfo>,
    autostart: bool,
    active_allowances: HashMap<String, u64>,
}

#[tauri::command]
async fn get_state(app: AppHandle, core: State<'_, Shared>) -> Result<DashState, String> {
    let c = core.lock().unwrap();
    let browsers = browser::BROWSERS
        .iter()
        .filter(|b| macos::app_installed(b.bundle))
        .map(|b| BrowserInfo {
            bundle: b.bundle,
            name: b.name,
            kind: b.kind,
            running: macos::is_running(b.bundle),
            status: c.browser_status.get(b.bundle).copied(),
        })
        .collect();
    let stats = c.stats.days.iter().rev().take(28).map(|(k, v)| (k.clone(), v.clone())).collect();
    let now = Instant::now();
    Ok(DashState {
        config: c.config.clone(),
        today: rules::date_key(&Local::now()),
        stats,
        accessibility: macos::ax_trusted(),
        browsers,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        active_allowances: c
            .allowances
            .iter()
            .filter(|(_, &t)| t > now)
            .map(|(k, t)| (k.clone(), (*t - now).as_secs()))
            .collect(),
    })
}

#[tauri::command]
async fn save_config(core: State<'_, Shared>, mut config: Config) -> Result<Config, String> {
    let stamp = Local::now().timestamp_millis();
    for (n, r) in config.rules.iter_mut().enumerate() {
        if r.id.trim().is_empty() {
            r.id = format!("r{stamp}{n}");
        }
        r.name = r.name.trim().to_string();
        r.sites = r
            .sites
            .iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        r.pause_seconds = r.pause_seconds.clamp(1, 600);
    }
    config.settings.session_options.retain(|&m| (1..=240).contains(&m));
    if config.settings.session_options.is_empty() {
        config.settings.session_options = rules::Settings::default().session_options;
    }
    let mut c = core.lock().unwrap();
    let ids: Vec<String> = config.rules.iter().map(|r| r.id.clone()).collect();
    c.allowances.retain(|k, _| ids.contains(k));
    c.config = config;
    c.save_config();
    Ok(c.config.clone())
}

#[tauri::command]
async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| e.to_string())?;
    Ok(al.is_enabled().unwrap_or(false))
}

#[tauri::command]
async fn open_privacy(pane: String) -> Result<(), String> {
    let anchor = match pane.as_str() {
        "accessibility" => {
            macos::ax_prompt();
            "Privacy_Accessibility"
        }
        _ => "Privacy_Automation",
    };
    std::process::Command::new("open")
        .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Asks a browser for its URL once — which makes macOS show the "allow Defixum to control …"
/// prompt the first time.
#[tauri::command]
async fn test_browser(core: State<'_, Shared>, runner: State<'_, Runner>, bundle: String) -> Result<String, String> {
    let known = browser::lookup(&bundle).ok_or("unknown browser")?;
    let status: &'static str = if known.kind == browser::Kind::Ax {
        if macos::ax_trusted() { "ok" } else { "denied" }
    } else if !macos::is_running(&bundle) {
        return Ok("not_running".into());
    } else {
        let src = browser::script_probe(&bundle, known.kind);
        let r = tauri::async_runtime::spawn_blocking({
            let runner = runner.0.clone();
            move || runner(Box::new(move || macos::applescript(&src)))
        })
        .await
        .ok()
        .flatten();
        match r {
            Some(macos::Script::Ok(_)) => "ok",
            Some(macos::Script::Denied) => "denied",
            _ => "error",
        }
    };
    core.lock().unwrap().browser_status.insert(bundle, status);
    Ok(status.into())
}

// ---------- setup ----------

const CALM_HTML: &str = include_str!("../calm.html");

fn data_dir(app: &AppHandle) -> PathBuf {
    let dir = std::env::var_os("DEFIXUM_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| app.path().app_data_dir().expect("no app data dir"));
    std::fs::create_dir_all(&dir).expect("cannot create data dir");
    dir
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            overlay_state,
            overlay_continue,
            overlay_back,
            get_state,
            save_config,
            set_autostart,
            open_privacy,
            test_browser
        ])
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            let dir = data_dir(&handle);
            let _ = LOG_PATH.set(dir.join("defixum.log"));
            let _ = std::fs::write(dir.join("calm.html"), CALM_HTML);

            let first_run = !dir.join("config.json").exists();
            let config: Config = read_json(&dir.join("config.json")).unwrap_or_else(Config::starter);
            let stats: Stats = read_json(&dir.join("stats.json")).unwrap_or_default();
            let core = Core {
                config,
                stats,
                stats_dirty: false,
                stats_saved: Instant::now(),
                allowances: HashMap::new(),
                expired: HashMap::new(),
                intercept: None,
                cooldown: HashMap::new(),
                last_target: None,
                browser_status: HashMap::new(),
                data_dir: dir.clone(),
            };
            if first_run {
                core.save_config();
            }
            let shared: Shared = Arc::new(Mutex::new(core));
            app.manage(shared.clone());

            let main_handle = handle.clone();
            let runner: Arc<MainRunner> = Arc::new(move |f| {
                let (tx, rx) = std::sync::mpsc::channel();
                main_handle.run_on_main_thread(move || { let _ = tx.send(f()); }).ok()?;
                rx.recv_timeout(Duration::from_secs(5)).ok()
            });
            app.manage(Runner(runner.clone()));

            // Windows
            let overlay = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
                .title("Defixum")
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .visible(false)
                .visible_on_all_workspaces(true)
                .effects(EffectsBuilder::new().effect(Effect::HudWindow).state(EffectState::Active).build())
                .build()?;
            raise_overlay(&overlay);

            let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Defixum")
                .inner_size(1040.0, 760.0)
                .min_inner_size(820.0, 600.0)
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true)
                .visible(first_run || std::env::var_os("DEFIXUM_SHOW").is_some())
                .build()?;
            let main_clone = main.clone();
            main.on_window_event(move |e| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                    api.prevent_close();
                    let _ = main_clone.hide();
                }
            });

            // Menu bar
            let open_item = MenuItem::with_id(app, "open", "Open Defixum…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Defixum", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &PredefinedMenuItem::separator(app)?, &quit_item])?;
            TrayIconBuilder::with_id("tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .tooltip("Defixum")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "open" => open_dashboard(app),
                    "quit" => {
                        if let Some(core) = app.try_state::<Shared>() {
                            core.lock().unwrap().save_stats();
                        }
                        app.exit(0)
                    }
                    _ => {}
                })
                .build(app)?;

            log(format!("started; data in {}", dir.display()));
            std::thread::spawn(move || monitor(handle, shared, runner));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Defixum")
        .run(|app, event| match event {
            tauri::RunEvent::Reopen { .. } => open_dashboard(app),
            tauri::RunEvent::Exit => {
                if let Some(core) = app.try_state::<Shared>() {
                    core.lock().unwrap().save_stats();
                }
            }
            _ => {}
        });
}

/// `defixum --probe`: print what Defixum sees in the frontmost browser, once a second.
pub fn probe(seconds: u32, bundle: Option<String>) {
    use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
    let direct: Arc<MainRunner> = Arc::new(|f| Some(f()));
    let mut ax = AxState::default();
    println!("accessibility trusted: {}", macos::ax_trusted());
    for _ in 0..seconds {
        // spin the run loop so NSWorkspace notices app switches
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, Duration::from_secs(1), false);
        let f = match &bundle {
            Some(b) => match macos::pid_of(b) {
                Some(pid) => macos::FrontApp { bundle: b.clone(), pid },
                None => { println!("{b} is not running"); continue }
            },
            None => match macos::frontmost() { Some(f) => f, None => continue },
        };
        let Some(k) = browser::lookup(&f.bundle) else {
            println!("{} (not a browser)", f.bundle);
            continue;
        };
        let t = Target { bundle: f.bundle.clone(), pid: f.pid, kind: k.kind };
        let started = Instant::now();
        let r = browser::fetch(&t, &mut ax, direct.as_ref());
        println!("{} [{:?}] {:?} ({:.1} ms)", k.name, k.kind, r, started.elapsed().as_secs_f64() * 1000.0);
    }
}
