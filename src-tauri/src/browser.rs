//! Which apps are browsers, and how to read / change the active tab's URL in each — without
//! a browser extension.
//!
//! * Safari and Chromium-family browsers answer AppleScript, so we ask them directly.
//! * Firefox-family browsers (Firefox, Zen, LibreWolf…) have no tab scripting, so we read the
//!   page's `AXURL` through the Accessibility API, and navigate by driving the address bar.

use crate::macos::{self, Element, Script};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Safari,
    Chromium,
    /// Read through Accessibility (Firefox family and anything else exposing AXWebArea).
    Ax,
}

pub struct Known {
    pub bundle: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

pub const BROWSERS: &[Known] = &[
    Known { bundle: "com.apple.Safari", name: "Safari", kind: Kind::Safari },
    Known { bundle: "com.apple.SafariTechnologyPreview", name: "Safari Technology Preview", kind: Kind::Safari },
    Known { bundle: "com.google.Chrome", name: "Google Chrome", kind: Kind::Chromium },
    Known { bundle: "com.google.Chrome.beta", name: "Chrome Beta", kind: Kind::Chromium },
    Known { bundle: "com.google.Chrome.dev", name: "Chrome Dev", kind: Kind::Chromium },
    Known { bundle: "com.google.Chrome.canary", name: "Chrome Canary", kind: Kind::Chromium },
    Known { bundle: "org.chromium.Chromium", name: "Chromium", kind: Kind::Chromium },
    Known { bundle: "company.thebrowser.Browser", name: "Arc", kind: Kind::Chromium },
    Known { bundle: "company.thebrowser.dia", name: "Dia", kind: Kind::Chromium },
    Known { bundle: "com.brave.Browser", name: "Brave", kind: Kind::Chromium },
    Known { bundle: "com.microsoft.edgemac", name: "Microsoft Edge", kind: Kind::Chromium },
    Known { bundle: "com.operasoftware.Opera", name: "Opera", kind: Kind::Chromium },
    Known { bundle: "com.vivaldi.Vivaldi", name: "Vivaldi", kind: Kind::Chromium },
    Known { bundle: "ai.perplexity.comet", name: "Comet", kind: Kind::Chromium },
    Known { bundle: "com.openai.atlas", name: "ChatGPT Atlas", kind: Kind::Chromium },
    Known { bundle: "app.zen-browser.zen", name: "Zen", kind: Kind::Ax },
    Known { bundle: "org.mozilla.firefox", name: "Firefox", kind: Kind::Ax },
    Known { bundle: "org.mozilla.firefoxdeveloperedition", name: "Firefox Developer Edition", kind: Kind::Ax },
    Known { bundle: "org.mozilla.nightly", name: "Firefox Nightly", kind: Kind::Ax },
    Known { bundle: "io.gitlab.librewolf-community", name: "LibreWolf", kind: Kind::Ax },
    Known { bundle: "net.waterfox.waterfox", name: "Waterfox", kind: Kind::Ax },
    Known { bundle: "org.mozilla.floorp", name: "Floorp", kind: Kind::Ax },
    Known { bundle: "com.kagi.kagimacOS", name: "Orion", kind: Kind::Ax },
];

pub fn lookup(bundle: &str) -> Option<&'static Known> {
    BROWSERS.iter().find(|b| b.bundle == bundle)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub bundle: String,
    pub pid: i32,
    pub kind: Kind,
}

/// AppleScript must run on the main thread; this hands it there (or runs it directly when
/// the caller already is the main thread, as in `--probe`).
pub type MainRunner = dyn Fn(Box<dyn FnOnce() -> Script + Send>) -> Option<Script> + Sync + Send;

fn script_get(t: &Target) -> String {
    let tab = if t.kind == Kind::Safari { "current tab" } else { "active tab" };
    format!(
        "with timeout of 2 seconds\n tell application id \"{}\"\n if (count of windows) is 0 then return \"\"\n return URL of {tab} of front window\n end tell\nend timeout",
        t.bundle
    )
}

fn script_set(t: &Target, url: &str) -> String {
    let tab = if t.kind == Kind::Safari { "current tab" } else { "active tab" };
    format!(
        "with timeout of 2 seconds\n tell application id \"{}\" to set URL of {tab} of front window to \"{}\"\nend timeout",
        t.bundle,
        url.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// Closes the active tab. If it is the window's only tab, a fresh empty tab is opened first so
/// the window (and the browser) stays and the user lands on the normal new-tab page.
fn script_close(t: &Target) -> String {
    let safari = t.kind == Kind::Safari;
    let tab = if safari { "current tab" } else { "active tab" };
    // Safari does not switch to a tab made by script; Chromium browsers do.
    let focus = if safari { " set current tab of win to newTab\n" } else { "" };
    format!(
        "with timeout of 2 seconds\n tell application id \"{}\"\n if (count of windows) is 0 then error \"no window\"\n set win to front window\n set oldTab to {tab} of win\n if (count of tabs of win) > 1 then\n close oldTab\n else\n set newTab to make new tab at end of tabs of win\n{focus} close oldTab\n end if\n end tell\nend timeout",
        t.bundle
    )
}

pub fn script_probe(bundle: &str, kind: Kind) -> String {
    script_get(&Target { bundle: bundle.into(), pid: 0, kind })
}

/// Per-thread helper state for the Accessibility path.
#[derive(Default)]
pub struct AxState {
    enabled_at: HashMap<i32, Instant>,
    last_scan: Option<Instant>,
    last_url: HashMap<i32, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Fetch {
    Url(String),
    /// Readable, but no page is showing (no windows, new tab being typed into, …).
    Nothing,
    Denied,
    Unavailable,
}

pub fn fetch(t: &Target, ax: &mut AxState, main: &MainRunner) -> Fetch {
    match t.kind {
        Kind::Safari | Kind::Chromium => {
            let src = script_get(t);
            match main(Box::new(move || macos::applescript(&src))) {
                Some(Script::Ok(u)) if u.is_empty() || u == "missing value" => Fetch::Nothing,
                Some(Script::Ok(u)) => Fetch::Url(u),
                Some(Script::Denied) => Fetch::Denied,
                _ => Fetch::Unavailable,
            }
        }
        Kind::Ax => {
            if !macos::ax_trusted() {
                return Fetch::Denied;
            }
            match ax_page_url(t.pid, ax) {
                Some(u) => Fetch::Url(u),
                None => Fetch::Nothing,
            }
        }
    }
}

/// Gecko builds its accessibility tree only once an assistive client asks for it.
fn ax_wake(app: &Element, pid: i32, ax: &mut AxState, force: bool) {
    let due = ax.enabled_at.get(&pid).map_or(true, |t| t.elapsed() > Duration::from_secs(20));
    if force || due {
        app.set_bool("AXManualAccessibility", true);
        app.set_bool("AXEnhancedUserInterface", true);
        ax.enabled_at.insert(pid, Instant::now());
    }
}

fn ax_page_url(pid: i32, ax: &mut AxState) -> Option<String> {
    let app = Element::application(pid)?;
    ax_wake(&app, pid, ax, false);

    // Fast path (~1 ms): walk up from the focused element; the outermost web area is the
    // top-level page (inner ones are iframes).
    if let Some(mut e) = app.element("AXFocusedUIElement") {
        let mut outer: Option<Element> = None;
        for _ in 0..80 {
            let role = e.role();
            if role == "AXWebArea" {
                outer = Some(e.clone());
            }
            if role == "AXWindow" || role == "AXApplication" {
                break;
            }
            match e.element("AXParent") {
                Some(p) => e = p,
                None => break,
            }
        }
        if let Some(url) = outer.and_then(|w| w.url()) {
            ax.last_url.insert(pid, url.clone());
            return Some(url);
        }
    }

    // Focus is in browser chrome (address bar, sidebar…). Scanning the window is ~100× the
    // cost, so do it at most every 2 s and reuse the last answer in between.
    if ax.last_scan.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
        return ax.last_url.get(&pid).cloned();
    }
    ax.last_scan = Some(Instant::now());
    let window = app.element("AXFocusedWindow").or_else(|| app.element("AXMainWindow"));
    let Some(window) = window else {
        ax.last_url.remove(&pid);
        return None;
    };
    let found = window.find(
        800,
        &|_, role| role == "AXWebArea",
        &|role| role != "AXWebArea" && role != "AXToolbar" && role != "AXButton" && role != "AXStaticText",
    );
    match found.and_then(|w| w.url()) {
        Some(url) => {
            ax.last_url.insert(pid, url.clone());
            Some(url)
        }
        None => {
            // An empty tree usually means Gecko accessibility went back to sleep.
            ax_wake(&app, pid, ax, true);
            ax.last_url.remove(&pid);
            None
        }
    }
}

/// Points the active tab somewhere else. For the Accessibility path the browser must already
/// be frontmost (we type into its address bar).
pub fn navigate(t: &Target, url: &str, main: &MainRunner) -> bool {
    match t.kind {
        Kind::Safari | Kind::Chromium => {
            let src = script_set(t, url);
            matches!(main(Box::new(move || macos::applescript(&src))), Some(Script::Ok(_)))
        }
        Kind::Ax => {
            let Some(app) = Element::application(t.pid) else { return false };
            let Some(window) = app.element("AXFocusedWindow").or_else(|| app.element("AXMainWindow")) else {
                return false;
            };
            let bar = window.find(
                1500,
                &|e, role| {
                    role == "AXComboBox"
                        || (role == "AXTextField"
                            && e.string("AXDescription").is_some_and(|d| d.to_lowercase().contains("address")))
                },
                &|role| role != "AXWebArea",
            );
            let Some(bar) = bar else { return false };
            bar.set_bool("AXFocused", true);
            std::thread::sleep(Duration::from_millis(80));
            if bar.set_string("AXValue", url) != 0 {
                return false;
            }
            std::thread::sleep(Duration::from_millis(80));
            macos::press_key(t.pid, 36); // Return
            true
        }
    }
}

/// Goes back one page in the active tab's history, if it has any: the way out of a site that
/// was reached by following a link in the same tab. Returns whether it went back, so the
/// caller can close the tab instead. Both browser families are driven through Accessibility,
/// which also tells us (enabled or not) whether there is anywhere to go back to.
pub fn go_back(t: &Target, main: &MainRunner) -> bool {
    if !macos::ax_trusted() {
        return false;
    }
    let Some(app) = Element::application(t.pid) else { return false };
    match t.kind {
        Kind::Ax => {
            // Firefox-family browsers have no Back menu item; the toolbar button carries the
            // DOM id "back-button" and is disabled when the tab has no history.
            let Some(window) = app.element("AXFocusedWindow").or_else(|| app.element("AXMainWindow")) else {
                return false;
            };
            let button = window.find(
                400,
                &|e, role| {
                    role == "AXButton"
                        && e.string("AXDOMIdentifier").as_deref() == Some("back-button")
                        && e.flag("AXEnabled") == Some(true)
                },
                &|role| role != "AXWebArea",
            );
            button.is_some_and(|b| b.press() == 0)
        }
        Kind::Safari | Kind::Chromium => {
            // History > Back (Cmd+[), disabled when there is nothing to go back to. The menu
            // shows the shortcut as typed on the current layout, hence the lookup.
            let Some(bar) = app.element("AXMenuBar") else { return false };
            let layout_key = match main(Box::new(|| Script::Ok(macos::key_char(33).unwrap_or_default()))) {
                Some(Script::Ok(c)) => c,
                _ => String::new(),
            };
            let item = bar.find(
                600,
                &|e, role| {
                    role == "AXMenuItem"
                        && e.string("AXMenuItemCmdChar")
                            .is_some_and(|c| c == "[" || macos::same_key(&c, &layout_key))
                        && e.int("AXMenuItemCmdModifiers") == Some(0)
                        && e.flag("AXEnabled") == Some(true)
                },
                &|role| matches!(role, "AXMenuBar" | "AXMenuBarItem" | "AXMenu" | "AXMenuItem"),
            );
            item.is_some_and(|i| i.press() == 0)
        }
    }
}

/// Closes the active tab without going through the page. Safari and Chromium are asked by
/// AppleScript; the Accessibility path presses the app's own "Close Tab" menu item, which is
/// the one bound to Cmd+W, so it works in any language. The browser should be frontmost.
pub fn close_tab(t: &Target, main: &MainRunner) -> bool {
    match t.kind {
        Kind::Safari | Kind::Chromium => {
            let src = script_close(t);
            matches!(main(Box::new(move || macos::applescript(&src))), Some(Script::Ok(_)))
        }
        Kind::Ax => {
            let Some(app) = Element::application(t.pid) else { return false };
            let Some(bar) = app.element("AXMenuBar") else { return false };
            // The menu reports the shortcut as typed on the current layout (Ü for W on Azeri).
            // The layout lookup is main-thread only, so it goes through the same runner as scripts.
            let layout_w = match main(Box::new(|| Script::Ok(macos::key_char(13).unwrap_or_default()))) {
                Some(Script::Ok(c)) => c,
                _ => String::new(),
            };
            // Only the menu bar is searched; menu items are reachable without opening a menu.
            let item = bar.find(
                400,
                &|e, role| {
                    role == "AXMenuItem"
                        && e.string("AXMenuItemCmdChar").is_some_and(|c| macos::same_key(&c, "w") || macos::same_key(&c, &layout_w))
                        // 0 is Command alone; Shift+Cmd+W (Close Window) is 1
                        && e.int("AXMenuItemCmdModifiers") == Some(0)
                        && e.flag("AXEnabled") == Some(true)
                },
                &|role| matches!(role, "AXMenuBar" | "AXMenuBarItem" | "AXMenu" | "AXMenuItem"),
            );
            item.is_some_and(|i| i.press() == 0)
        }
    }
}
