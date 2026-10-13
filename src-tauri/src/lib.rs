mod bedtime;
mod browser;
mod macos;
mod prompts;
mod rules;
mod switcher;
mod weather;

use browser::{AxState, Fetch, MainRunner, Target};
use chrono::{Local, TimeZone, Timelike};
use prompts::Prompt;
use rules::{Config, Countdown, DayStats, Reason, Stats};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

const TICK: Duration = Duration::from_millis(900);
const STATS_KEEP_DAYS: usize = 120;
/// How long a finished pause stays finished after the user leaves the tab or the browser.
const WAITED_GRACE: Duration = Duration::from_secs(10 * 60);

// ---------- menu bar ----------

/// Time left as the menu bar shows it: whole minutes rounded up, seconds in the last minute.
fn short_left(d: Duration) -> String {
    let secs = d.as_secs() + u64::from(d.subsec_nanos() > 0);
    if secs < 60 {
        return format!("{secs}s");
    }
    let m = secs.div_ceil(60);
    match (m / 60, m % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Unlocked rules that haven't run out yet, soonest first: (rule id, rule name, time left).
fn open_windows(c: &Core) -> Vec<(String, String, Duration)> {
    let now = Instant::now();
    let mut open: Vec<(String, String, Duration)> = c
        .allowances
        .iter()
        .filter(|(_, &until)| until > now)
        .map(|(id, &until)| {
            let name = c.config.rules.iter().find(|r| &r.id == id).map_or("A rule", |r| r.name.as_str());
            (id.clone(), name.to_string(), until - now)
        })
        .collect();
    open.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1)));
    open
}

fn tray_menu(app: &AppHandle, lines: &[String]) -> tauri::Result<(Menu<tauri::Wry>, Vec<MenuItem<tauri::Wry>>)> {
    let menu = Menu::new(app)?;
    let mut items = vec![];
    for (n, line) in lines.iter().enumerate() {
        let item = MenuItem::with_id(app, format!("left-{n}"), line, false, None::<&str>)?;
        menu.append(&item)?;
        items.push(item);
    }
    if !lines.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "open", "Open Defixum…", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit Defixum", true, None::<&str>)?)?;
    Ok((menu, items))
}

/// What the tray currently shows, so it's only touched when something changed.
#[derive(Default)]
struct TrayView {
    title: Option<String>,
    rule_ids: Vec<String>,
    lines: Vec<String>,
    items: Vec<MenuItem<tauri::Wry>>,
}

/// Puts the soonest-ending unlock next to the menu bar icon and lists every unlock in its menu.
fn refresh_tray(app: &AppHandle, core: &Shared, view: &mut TrayView) {
    let open = open_windows(&core.lock().unwrap());
    let title = open.first().map(|(_, _, left)| short_left(*left));
    let rule_ids: Vec<String> = open.iter().map(|(id, _, _)| id.clone()).collect();
    let lines: Vec<String> = open.iter().map(|(_, name, left)| format!("{name}: {} left", short_left(*left))).collect();
    let Some(tray) = app.tray_by_id("tray") else { return };

    if title != view.title {
        let _ = tray.set_title(title.as_deref());
        // text to the left of the logo (tray-icon puts it on the right)
        let _ = tray.with_inner_tray_icon(|t| {
            if let Some(item) = t.ns_status_item() {
                unsafe {
                    let button: Option<objc2::rc::Retained<objc2::runtime::AnyObject>> = objc2::msg_send![&*item, button];
                    if let Some(button) = button {
                        let _: () = objc2::msg_send![&*button, setImagePosition: 3usize]; // NSImageRight
                    }
                }
            }
        });
        view.title = title;
    }
    if rule_ids != view.rule_ids {
        // a different set of unlocks: rebuild the menu
        if let Ok((menu, items)) = tray_menu(app, &lines) {
            let _ = tray.set_menu(Some(menu));
            view.items = items;
            view.rule_ids = rule_ids;
            view.lines = lines;
        }
    } else if lines != view.lines {
        // same unlocks, new times: update in place so an open menu stays open and current
        for (item, line) in view.items.iter().zip(&lines) {
            let _ = item.set_text(line);
        }
        view.lines = lines;
    }
}

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
    countdown: Countdown,
    /// Chosen once when the pause starts, so a re-render shows the same text. Only for pauses.
    prompt: Option<Prompt>,
    opens_today: u32,
    minutes_today: u64,
    daily_opens: Option<u32>,
    daily_minutes: Option<u32>,
    session_options: Vec<u32>,
    until: Option<String>,
    sky: String,
    /// the weather outside, when that setting is on and a recent reading exists
    weather: Option<weather::Weather>,
    /// The pause was already served earlier (see `Core::waited`), so the overlay skips the countdown.
    waited: bool,
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
    /// rule id → when a finished pause was dismissed by switching tab or app (and its prompt),
    /// so coming back within `WAITED_GRACE` doesn't make the user wait again
    waited: HashMap<String, (Instant, Option<Prompt>)>,
    /// after "go back", don't re-trigger for a moment while the tab navigates away
    cooldown: HashMap<String, Instant>,
    last_target: Option<Target>,
    /// what the previous pause said (`Prompt::memo`), so the next one doesn't repeat it
    last_prompt: Option<String>,
    /// bundle id → "ok" | "denied" | "error"
    browser_status: HashMap<String, &'static str>,
    /// which bedtime reminders have been shown, and the night that was muted
    bedtime_log: bedtime::Log,
    /// the bedtime reminder on screen
    bedtime_up: Option<BedtimeUp>,
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

    fn save_bedtime(&self) {
        write_json(&self.data_dir.join("bedtime.json"), &self.bedtime_log);
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

/// Covers the monitor under the cursor (or the primary one) with the window.
fn cover_monitor(app: &AppHandle, w: &WebviewWindow) {
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
}

/// Puts a full-screen window back over a whole screen if it no longer matches one, which happens
/// when screens come and go: waking from sleep, a lid closing, a monitor plugged in. Otherwise
/// its bottom band, where the buttons are, can end up past the edge of the screen.
fn refit(app: &AppHandle, label: &str) {
    let Some(w) = app.get_webview_window(label) else { return };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { return };
    let close = |a: i32, b: i32| (a - b).abs() <= 2;
    let fits = app.available_monitors().unwrap_or_default().iter().any(|m| {
        close(m.position().x, pos.x)
            && close(m.position().y, pos.y)
            && close(m.size().width as i32, size.width as i32)
            && close(m.size().height as i32, size.height as i32)
    });
    if !fits {
        log(format!("{label}: screens changed, refitting the window"));
        cover_monitor(app, &w);
    }
}

/// Shows the window and makes its web view the key view.
fn show_key(w: &WebviewWindow) {
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

fn overlay_show(app: &AppHandle, payload: &Intercept) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    cover_monitor(app, &w);
    let _ = app.emit_to("overlay", "intercept", payload);
    show_key(&w);
}

fn overlay_hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = app.emit_to("overlay", "dismiss", ());
        let _ = w.hide();
    }
}

/// Float a full-screen window above everything, including other apps' full-screen spaces.
fn raise_window(w: &WebviewWindow, level: isize) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    if let Ok(ptr) = w.ns_window() {
        let win: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
        win.setLevel(level);
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

// ---------- Cmd+Tab limit ----------

const TOAST_SIZE: (f64, f64) = (360.0, 68.0);
/// How long the toast stays up after the last refused Cmd+Tab.
const TOAST_FOR: Duration = Duration::from_secs(5);
static TOAST_SHOWN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Serialize, Clone)]
struct Toast {
    /// end of the cooldown, unix milliseconds
    until: i64,
    limit: u32,
}

/// Shows the toast near the top of the screen under the pointer, without taking focus, and
/// hides it again unless another refusal has refreshed it in the meantime. Main thread only.
fn toast_show(app: &AppHandle, until: i64, limit: u32) {
    let Some(w) = app.get_webview_window("toast") else { return };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let width = (TOAST_SIZE.0 * scale) as i32;
        let x = m.position().x + (m.size().width as i32 - width) / 2;
        let y = m.position().y + (56.0 * scale) as i32;
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
    }
    let _ = app.emit_to("toast", "toast", Toast { until, limit });
    if let Ok(ptr) = w.ns_window() {
        let win: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        win.orderFrontRegardless();
    }
    let shown = TOAST_SHOWN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(TOAST_FOR);
        if TOAST_SHOWN.load(std::sync::atomic::Ordering::SeqCst) == shown {
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = a.get_webview_window("toast") {
                    let _ = w.hide();
                }
            });
        }
    });
}

// ---------- bedtime reminder ----------

/// What the bedtime screen is told.
#[derive(Clone, Serialize)]
struct Bedtime {
    /// which reminder: minutes before the sleep time, 0 at it
    offset: u32,
    minutes_left: i64,
    /// the sleep time as set, "23:00"
    bedtime: String,
    /// unix seconds of the sleep time this reminder counts toward
    bedtime_ts: i64,
    sky: String,
    /// the weather outside, when that setting is on and a recent reading exists
    weather: Option<weather::Weather>,
}

/// The reminder on screen, and where to put the user back when it goes.
struct BedtimeUp {
    shown: Bedtime,
    /// the night it belongs to, for "Don't remind again today"
    night: String,
    return_to: Option<i32>,
}

fn minutes_until(ts: i64) -> i64 {
    (ts - Local::now().timestamp() + 59).div_euclid(60).max(0)
}

fn bedtime_show(app: &AppHandle, payload: &Bedtime) {
    let Some(w) = app.get_webview_window("bedtime") else { return };
    cover_monitor(app, &w);
    let _ = app.emit_to("bedtime", "bedtime", payload);
    show_key(&w);
}

/// Takes the reminder off screen and puts the user back in the app they were using. With `mute`,
/// the rest of this night's reminders are off too.
fn bedtime_dismiss(app: &AppHandle, core: &Shared, mute: bool) {
    let (up, pause_up) = {
        let mut c = core.lock().unwrap();
        let Some(up) = c.bedtime_up.take() else { return };
        if mute {
            c.bedtime_log.mute(&up.night);
            c.save_bedtime();
        }
        (up, c.intercept.is_some())
    };
    log(format!("bedtime reminder ({} min) {}", up.shown.offset, if mute { "muted for tonight" } else { "dismissed" }));
    if let Some(w) = app.get_webview_window("bedtime") {
        let _ = app.emit_to("bedtime", "bedtime_hide", ());
        let _ = w.hide();
    }
    match up.return_to {
        Some(pid) => macos::activate(pid),
        // a pause was underneath: give it the keyboard back
        None if pause_up => {
            if let Some(w) = app.get_webview_window("overlay") {
                show_key(&w);
            }
        }
        None => {}
    }
}

/// A reminder still up this long after bedtime was left behind (the Mac slept, the user walked
/// away); it is taken down rather than greeting them in the morning.
const BEDTIME_STALE: i64 = 3 * 3600;

/// Shows the reminder that is due now, if there is one. Called every few seconds.
fn bedtime_tick(app: &AppHandle, core: &Shared) {
    let stale = core
        .lock()
        .unwrap()
        .bedtime_up
        .as_ref()
        .is_some_and(|u| Local::now().timestamp() - u.shown.bedtime_ts > BEDTIME_STALE);
    if stale {
        log("bedtime reminder left up for hours; taking it down");
        bedtime_dismiss(app, core, false);
    }
    // nobody would see it; a reminder still in its five minutes shows once the screen is on
    if macos::display_asleep() {
        return;
    }
    let (payload, night) = {
        let mut c = core.lock().unwrap();
        let s = &c.config.settings;
        if !s.bedtime {
            return;
        }
        let Some(at) = bedtime::parse_time(&s.bedtime_at) else { return };
        let Some(due) = bedtime::due(Local::now().naive_local(), at, &c.bedtime_log) else { return };
        let Some(ts) = Local.from_local_datetime(&due.bedtime).earliest().map(|t| t.timestamp()) else { return };
        let payload = Bedtime {
            offset: due.offset,
            minutes_left: minutes_until(ts),
            bedtime: s.bedtime_at.clone(),
            bedtime_ts: ts,
            sky: s.sky.clone(),
            weather: if s.weather { weather::current() } else { None },
        };
        c.bedtime_log.record(&due.night, due.offset);
        c.save_bedtime();
        (payload, due.night)
    };
    let own_pid = std::process::id() as i32;
    let front = macos::frontmost().map(|f| f.pid).filter(|&p| p != own_pid);
    {
        let mut c = core.lock().unwrap();
        // a newer reminder replaces one still on screen, which already knows where to return to
        let return_to = c.bedtime_up.as_ref().map_or(front, |u| u.return_to);
        c.bedtime_up = Some(BedtimeUp { shown: payload.clone(), night, return_to });
    }
    log(format!("bedtime reminder: {} min before {}", payload.offset, payload.bedtime));
    bedtime_show(app, &payload);
}

#[tauri::command]
async fn bedtime_state(core: State<'_, Shared>) -> Result<Option<Bedtime>, String> {
    Ok(core.lock().unwrap().bedtime_up.as_ref().map(|u| Bedtime {
        minutes_left: minutes_until(u.shown.bedtime_ts),
        ..u.shown.clone()
    }))
}

/// "Got it".
#[tauri::command]
async fn bedtime_ok(app: AppHandle, core: State<'_, Shared>) -> Result<(), String> {
    bedtime_dismiss(&app, &core, false);
    Ok(())
}

/// "Don't remind again today".
#[tauri::command]
async fn bedtime_mute(app: AppHandle, core: State<'_, Shared>) -> Result<(), String> {
    bedtime_dismiss(&app, &core, true);
    Ok(())
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
    let mut tray = TrayView::default();
    loop {
        std::thread::sleep(TICK);
        let dt = last.elapsed().min(Duration::from_secs(3));
        last = Instant::now();
        refresh_tray(&app, &core, &mut tray);

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
                dismiss_intercept(&mut core.lock().unwrap());
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

/// Should dismissing an intercept (focus or URL moved on) be remembered? Only a pause that
/// could be continued and has already run its course: an unfinished one starts over.
fn should_remember(reason: Reason, shown_for: Duration, pause_seconds: u32) -> bool {
    reason.can_continue() && shown_for >= Duration::from_secs(pause_seconds as u64)
}

/// Is a remembered finished pause still good for a new intercept with this reason?
fn waited_applies(reason: Reason, age: Duration) -> bool {
    reason.can_continue() && age < WAITED_GRACE
}

/// Drop the current intercept because the user moved on without choosing continue or back.
/// Returns whether there was one (i.e. whether the overlay needs hiding).
fn dismiss_intercept(c: &mut Core) -> bool {
    let Some(i) = c.intercept.take() else { return false };
    if should_remember(i.reason, i.shown_at.elapsed(), i.pause_seconds) {
        c.waited.insert(i.rule_id, (Instant::now(), i.prompt));
    }
    true
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
    c.waited.retain(|_, (t, _)| t.elapsed() < WAITED_GRACE);

    let local = Local::now();
    let verdict = url.and_then(|u| {
        rules::evaluate(&c.config, &c.stats, &local, u, &|id| c.allowances.contains_key(id))
    });
    let Some(v) = verdict else {
        return if dismiss_intercept(c) { UiAction::Hide } else { UiAction::None };
    };
    let rule = c.config.rules[v.rule_index].clone();
    let today = rules::date_key(&local);

    let Some(mut reason) = v.reason else {
        // Inside a granted window: count the time while the user is actually there.
        if focused && macos::idle_seconds() < c.config.settings.idle_seconds as f64 {
            c.stats.day_mut(&today, &rule.id).seconds += dt.as_secs_f64().round() as u64;
            c.stats_dirty = true;
        }
        return if dismiss_intercept(c) { UiAction::Hide } else { UiAction::None };
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
    let site = rules::host_of(url.unwrap_or_default());
    let mut options: Vec<u32> = c.config.settings.session_options.iter().copied().filter(|&m| m > 0).collect();
    options.sort_unstable();
    options.dedup();
    options.truncate(4);
    // A pause the user already sat through, shortly ago: same prompt, nothing left to wait for.
    let waited = c
        .waited
        .get(&rule.id)
        .is_some_and(|(t, _)| waited_applies(reason, t.elapsed()))
        .then(|| c.waited.remove(&rule.id))
        .flatten();
    // switching straight from another rule's site: that intercept is dismissed like any other
    dismiss_intercept(c);
    let pause_seconds = rule.pause_seconds.clamp(1, 600);
    let prompt = if let Some((_, p)) = &waited {
        p.clone()
    } else {
        let prompt = reason.can_continue().then(|| {
            let ctx = prompts::Ctx {
                site: &site,
                hour: local.hour(),
                opens_today: day.opens,
                minutes_today: day.seconds / 60,
                longest_session: c.config.settings.session_options.iter().copied().max().unwrap_or(15),
            };
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64);
            prompts::pick(&rule.prompts, &ctx, c.last_prompt.as_deref(), seed)
        });
        if let Some(p) = &prompt {
            c.last_prompt = Some(p.memo());
        }
        prompt
    };
    // Backdated so `overlay_continue` sees a finished pause (if the clock can't go back that far,
    // the pause just runs again on the Rust side).
    let shown_at = if waited.is_some() {
        now.checked_sub(Duration::from_secs(pause_seconds as u64)).unwrap_or(now)
    } else {
        now
    };
    let i = Intercept {
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        site,
        reason,
        pause_seconds,
        countdown: rule.countdown,
        prompt,
        opens_today: day.opens,
        minutes_today: day.seconds / 60,
        daily_opens: rule.daily_opens,
        daily_minutes: rule.daily_minutes,
        session_options: options,
        until,
        sky: c.config.settings.sky.clone(),
        weather: if c.config.settings.weather { weather::current() } else { None },
        waited: waited.is_some(),
        shown_at,
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

/// The weather outside for the sky, or nothing when the setting is off or there is no recent reading.
#[tauri::command]
async fn weather_now(core: State<'_, Shared>) -> Result<Option<weather::Weather>, String> {
    let on = core.lock().unwrap().config.settings.weather;
    Ok(if on { weather::current() } else { None })
}

#[tauri::command]
async fn overlay_continue(
    app: AppHandle,
    core: State<'_, Shared>,
    minutes: u32,
    typed: Option<String>,
) -> Result<(), String> {
    let pid = {
        let mut c = core.lock().unwrap();
        let i = c.intercept.as_ref().ok_or("no pause in progress")?;
        if !i.reason.can_continue() {
            return Err("this site is blocked right now".into());
        }
        if i.shown_at.elapsed().as_secs_f64() + 0.5 < i.pause_seconds as f64 {
            return Err("the pause isn't over yet".into());
        }
        if let Some(p) = &i.prompt {
            if !prompts::unlocks(p, &typed.unwrap_or_default()) {
                return Err("Type the sentence exactly as shown to unlock.".into());
            }
        }
        let i = c.intercept.take().unwrap();
        let minutes = minutes.clamp(1, 240);
        c.allowances.insert(i.rule_id.clone(), Instant::now() + Duration::from_secs(minutes as u64 * 60));
        c.expired.remove(&i.rule_id);
        c.waited.remove(&i.rule_id);
        c.stats.day_mut(&rules::date_key(&Local::now()), &i.rule_id).opens += 1;
        c.save_stats();
        log(format!("continue {} for {minutes} min", i.site));
        i.target.pid
    };
    overlay_hide(&app);
    macos::activate(pid);
    Ok(())
}

/// Cmd+W in the dashboard: its close button only hides it too.
#[tauri::command]
fn close_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
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
        c.waited.remove(&i.rule_id);
        c.cooldown.insert(i.rule_id.clone(), Instant::now() + Duration::from_secs(3));
        log(format!("back from {}", i.site));
        (i.target, format!("{}#{}", c.calm_url(), i.site))
    };
    overlay_hide(&app);
    macos::activate(target.pid);
    let runner = runner.0.clone();
    // Leave the site by going back to the page the tab came from; a tab with no history is
    // closed. If the browser won't (no permission, no menu item, script error), send it to the
    // calm page instead so the site still goes away.
    let how = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(250));
        if browser::go_back(&target, runner.as_ref()) {
            "went back"
        } else if browser::close_tab(&target, runner.as_ref()) {
            "closed"
        } else if browser::navigate(&target, &calm, runner.as_ref()) {
            "calm page"
        } else {
            "none"
        }
    })
    .await
    .unwrap_or("none");
    match how {
        "went back" => log("went back to the previous page"),
        "closed" => log("closed the tab"),
        "calm page" => log("could not close the tab, sent it to the calm page"),
        _ => log("could not close or navigate the tab away"),
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
async fn save_config(app: AppHandle, core: State<'_, Shared>, mut config: Config) -> Result<Config, String> {
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
    if !["time", "light", "dark"].contains(&config.settings.sky.as_str()) {
        config.settings.sky = "time".into();
    }
    let bedtime_at = bedtime::parse_time(&config.settings.bedtime_at)
        .unwrap_or_else(|| bedtime::parse_time(&rules::Settings::default().bedtime_at).unwrap());
    config.settings.bedtime_at = bedtime_at.format("%H:%M").to_string();
    config.settings.switches_per_minute = config.settings.switches_per_minute.clamp(1, 60);
    config.settings.switch_cooldown_seconds = config.settings.switch_cooldown_seconds.clamp(5, 3600);
    let s = &config.settings;
    switcher::sync(s.switch_limit, s.switches_per_minute, s.switch_cooldown_seconds);
    let mut c = core.lock().unwrap();
    let ids: Vec<String> = config.rules.iter().map(|r| r.id.clone()).collect();
    c.allowances.retain(|k, _| ids.contains(k));
    // turned on, or moved: whatever has already passed tonight is not owed
    let s = &c.config.settings;
    if config.settings.bedtime && (!s.bedtime || s.bedtime_at != config.settings.bedtime_at) {
        c.bedtime_log.reschedule(Local::now().naive_local(), bedtime_at);
        c.save_bedtime();
    }
    let turned_off = !config.settings.bedtime && c.bedtime_up.is_some();
    c.config = config;
    c.save_config();
    let saved = c.config.clone();
    drop(c);
    if turned_off {
        bedtime_dismiss(&app, &core, false);
    }
    Ok(saved)
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
        // Tauri's default macOS menu binds Cmd+W (Close Window) and Cmd+H (Hide), which AppKit
        // handles before the page sees the key. The overlay needs Cmd+W as "close the tab", so the
        // menu is just Quit and Edit; Edit is kept because Cmd+A/C/X/Z in the text fields run
        // through it.
        .menu(|app| {
            Menu::with_items(
                app,
                &[
                    &Submenu::with_items(app, "Defixum", true, &[&PredefinedMenuItem::quit(app, None)?])?,
                    &Submenu::with_items(
                        app,
                        "Edit",
                        true,
                        &[
                            &PredefinedMenuItem::undo(app, None)?,
                            &PredefinedMenuItem::redo(app, None)?,
                            &PredefinedMenuItem::separator(app)?,
                            &PredefinedMenuItem::cut(app, None)?,
                            &PredefinedMenuItem::copy(app, None)?,
                            &PredefinedMenuItem::paste(app, None)?,
                            &PredefinedMenuItem::select_all(app, None)?,
                        ],
                    )?,
                ],
            )
        })
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            overlay_state,
            weather_now,
            overlay_continue,
            overlay_back,
            close_main,
            get_state,
            save_config,
            set_autostart,
            open_privacy,
            test_browser,
            bedtime_state,
            bedtime_ok,
            bedtime_mute
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
                waited: HashMap::new(),
                cooldown: HashMap::new(),
                last_target: None,
                last_prompt: None,
                browser_status: HashMap::new(),
                bedtime_log: read_json(&dir.join("bedtime.json")).unwrap_or_default(),
                bedtime_up: None,
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
            raise_window(&overlay, 101); // NSPopUpMenuWindowLevel
            // The overlay is reused, never closed. If something still asks to close it (a stray
            // performClose), that counts as "close the tab", same as Esc.
            let overlay_app = handle.clone();
            overlay.on_window_event(move |e| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                    api.prevent_close();
                    let app = overlay_app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = overlay_back(app.clone(), app.state(), app.state()).await;
                    });
                }
            });

            // The bedtime reminder is the same forest in a second window, one level above the
            // overlay so it also covers a pause.
            let bedtime_win = WebviewWindowBuilder::new(app, "bedtime", WebviewUrl::App("overlay.html?mode=bedtime".into()))
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
            raise_window(&bedtime_win, 102);
            // Reused, never closed; a stray close request counts as "Got it".
            let bedtime_app = handle.clone();
            bedtime_win.on_window_event(move |e| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                    api.prevent_close();
                    bedtime_dismiss(&bedtime_app, &bedtime_app.state::<Shared>(), false);
                }
            });

            // A small note under the menu bar when Cmd+Tab is refused. It never takes focus or clicks.
            let toast = WebviewWindowBuilder::new(app, "toast", WebviewUrl::App("toast.html".into()))
                .title("Defixum")
                .inner_size(TOAST_SIZE.0, TOAST_SIZE.1)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .focused(false)
                .focusable(false)
                .visible(false)
                .visible_on_all_workspaces(true)
                .effects(EffectsBuilder::new().effect(Effect::HudWindow).state(EffectState::Active).radius(16.0).build())
                .build()?;
            raise_window(&toast, 103);
            let _ = toast.set_ignore_cursor_events(true);
            let toast_app = handle.clone();
            switcher::init(move |until, limit| {
                let app = toast_app.clone();
                let ms = Local::now().timestamp_millis() + until.saturating_duration_since(Instant::now()).as_millis() as i64;
                let _ = toast_app.run_on_main_thread(move || toast_show(&app, ms, limit));
            });

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

            // Menu bar (the monitor keeps its title and unlock list current)
            let (menu, _) = tray_menu(app.handle(), &[])?;
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
            let weather_core = shared.clone();
            let bedtime_core = shared.clone();
            let bedtime_handle = handle.clone();
            std::thread::spawn(move || monitor(handle, shared, runner));
            // bedtime reminders and screen changes: looks every couple of seconds, so a Mac waking
            // from sleep catches up quickly
            std::thread::spawn(move || loop {
                bedtime_tick(&bedtime_handle, &bedtime_core);
                if !macos::display_asleep() {
                    refit(&bedtime_handle, "bedtime");
                    refit(&bedtime_handle, "overlay");
                }
                // also starts the Cmd+Tab limit once Accessibility is granted
                let s = bedtime_core.lock().unwrap().config.settings.clone();
                switcher::sync(s.switch_limit, s.switches_per_minute, s.switch_cooldown_seconds);
                std::thread::sleep(Duration::from_secs(2));
            });
            // keeps a recent weather reading while the setting is on; checks twice a minute so
            // turning it on, or waking from sleep, catches up quickly
            std::thread::spawn(move || loop {
                if weather_core.lock().unwrap().config.settings.weather {
                    weather::refresh_if_due();
                }
                std::thread::sleep(Duration::from_secs(30));
            });
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

#[cfg(test)]
mod tests {
    use super::*;

    const SECS: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn menu_bar_time_rounds_up_to_minutes() {
        assert_eq!(short_left(Duration::from_millis(400)), "1s");
        assert_eq!(short_left(SECS(45)), "45s");
        assert_eq!(short_left(Duration::from_millis(59_200)), "1m");
        assert_eq!(short_left(SECS(60)), "1m");
        assert_eq!(short_left(SECS(61)), "2m");
        assert_eq!(short_left(SECS(13 * 60)), "13m");
        assert_eq!(short_left(SECS(60 * 60)), "1h");
        assert_eq!(short_left(SECS(60 * 60 + 1)), "1h 1m");
        assert_eq!(short_left(SECS(150 * 60)), "2h 30m");
    }

    #[test]
    fn unfinished_pause_is_not_remembered() {
        assert!(!should_remember(Reason::Pause, SECS(14), 15));
        assert!(!should_remember(Reason::Pause, Duration::ZERO, 15));
    }

    #[test]
    fn finished_pause_is_remembered() {
        assert!(should_remember(Reason::Pause, SECS(15), 15));
        assert!(should_remember(Reason::Expired, SECS(40), 15));
    }

    #[test]
    fn remembered_pause_expires_after_the_grace() {
        assert!(waited_applies(Reason::Pause, SECS(0)));
        assert!(waited_applies(Reason::Expired, WAITED_GRACE - SECS(1)));
        assert!(!waited_applies(Reason::Pause, WAITED_GRACE));
        assert!(!waited_applies(Reason::Pause, WAITED_GRACE + SECS(1)));
    }

    #[test]
    fn blocks_and_limits_ignore_waiting() {
        for r in [Reason::Blocked, Reason::OpenLimit, Reason::TimeLimit] {
            assert!(!should_remember(r, SECS(600), 15));
            assert!(!waited_applies(r, SECS(1)));
        }
    }
}
