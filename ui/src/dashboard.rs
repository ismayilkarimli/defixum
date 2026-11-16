//! The settings window. Holds the config in memory and saves it shortly after each edit.

use std::cell::RefCell;

use serde_json::json;
use wasm_bindgen::JsCast;
use web_sys::{ClipboardEvent, Event, HtmlElement, HtmlInputElement, KeyboardEvent, MouseEvent};

use crate::dom::{self, h, s, B};
use crate::sky::{self, Reading};
use crate::tauri;
use crate::types::*;

const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const FULL_DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

#[derive(Default)]
struct App {
    /// last get_state result
    state: Option<DashState>,
    /// the working copy; the source of truth, never swapped for what the app sends back
    config: Option<Config>,
    /// the rule in the editor
    selected: Option<String>,
    save_timer: Option<i32>,
    flash_timer: Option<i32>,
    /// the weather reading, when the Weather setting is on
    outside: Option<Reading>,
    /// the weekday picked in the bedtime week, Monday = 0
    bed_day: Option<usize>,
    bar_seed: u32,
    treeline_timer: Option<i32>,
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App::default());
}

fn app<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(&mut a.borrow_mut()))
}

/// Changes the working config (nothing before the first load).
fn cfg<R>(f: impl FnOnce(&mut Config) -> R) -> Option<R> {
    app(|a| a.config.as_mut().map(f))
}

fn settings() -> Settings {
    app(|a| a.config.as_ref().map(|c| c.settings.clone()).unwrap_or_default())
}

fn with_rule<R>(id: &str, f: impl FnOnce(&mut Rule) -> R) -> Option<R> {
    cfg(|c| c.rules.iter_mut().find(|r| r.id == id).map(f)).flatten()
}

fn today_stats(id: &str) -> DayStats {
    app(|a| {
        let st = a.state.as_ref()?;
        st.stats.get(&st.today)?.get(id).copied()
    })
    .unwrap_or_default()
}

fn times(n: u32) -> String {
    match n {
        1 => "once".into(),
        2 => "twice".into(),
        n => format!("{n} times"),
    }
}

fn fmt_minutes(sec: u64) -> String {
    let m = (sec as f64 / 60.0).round() as u64;
    if m < 60 {
        format!("{m} min")
    } else {
        format!("{} h {} min", m / 60, m % 60)
    }
}

fn input_el(e: &HtmlElement) -> HtmlInputElement {
    e.clone().unchecked_into()
}

fn inputs(sel: &str) -> Vec<HtmlInputElement> {
    dom::qa(sel).into_iter().map(|e| e.unchecked_into()).collect()
}

fn num(v: &str) -> f64 {
    let t = v.trim();
    if t.is_empty() {
        0.0
    } else {
        t.parse().unwrap_or(f64::NAN)
    }
}

/// `+value || fallback`, as the old page read number fields.
fn num_or(v: &str, fallback: f64) -> f64 {
    let n = num(v);
    if n.is_nan() || n == 0.0 {
        fallback
    } else {
        n
    }
}

// ---------- the app, or a stand-in for design work in a plain browser ----------

mod api {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static MOCK: RefCell<Option<Config>> = const { RefCell::new(None) };
    }

    fn date_key(d: &js_sys::Date) -> String {
        format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
    }

    fn mock_config() -> Config {
        MOCK.with(|m| {
            m.borrow_mut()
                .get_or_insert_with(|| {
                    let mut s = Settings::default();
                    s.sky = dom::param("sky").unwrap_or_else(|| "time".into());
                    let week = [true, true, true, true, true, false, false];
                    Config {
                        rules: vec![
                            Rule {
                                id: "starter".into(),
                                name: "Social & video".into(),
                                sites: ["youtube.com", "reddit.com", "x.com", "instagram.com"].map(String::from).to_vec(),
                                countdown: "fill".into(),
                                prompts: vec!["reflect".into(), "cost".into()],
                                daily_opens: Some(5),
                                daily_minutes: Some(30),
                                schedule: Schedule { enabled: false, days: week, start: "09:00".into(), end: "17:00".into() },
                                ..Rule::default()
                            },
                            Rule {
                                id: "work".into(),
                                name: "Work hours".into(),
                                sites: vec!["news.ycombinator.com".into()],
                                mode: "block".into(),
                                schedule: Schedule { enabled: true, days: week, start: "09:00".into(), end: "17:00".into() },
                                ..Rule::default()
                            },
                        ],
                        settings: s,
                        rest: Default::default(),
                    }
                })
                .clone()
        })
    }

    fn mock_state() -> DashState {
        let today = js_sys::Date::new_0();
        let mut stats = std::collections::BTreeMap::new();
        for i in 0..7u32 {
            let d = js_sys::Date::new_with_year_month_day(today.get_full_year(), today.get_month() as i32, today.get_date() as i32 - i as i32);
            let (opens, skips) = (1 + i % 3, 2 + (i * 3) % 4);
            let day = DayStats { intercepts: opens + skips + (i * 2) % 3, opens, skips, seconds: 600 + i as u64 * 420 };
            stats.insert(date_key(&d), std::collections::HashMap::from([("starter".to_string(), day)]));
        }
        let b = |bundle: &str, name: &str, kind: &str, running, status: Option<&str>| BrowserInfo {
            bundle: bundle.into(),
            name: name.into(),
            kind: kind.into(),
            running,
            status: status.map(String::from),
        };
        DashState {
            config: mock_config(),
            today: date_key(&today),
            stats,
            accessibility: true,
            autostart: false,
            browsers: vec![
                b("app.zen-browser.zen", "Zen", "ax", true, Some("ok")),
                b("com.google.Chrome", "Google Chrome", "chromium", false, None),
                b("company.thebrowser.Browser", "Arc", "chromium", true, Some("denied")),
                b("com.apple.Safari", "Safari", "safari", true, None),
            ],
        }
    }

    pub async fn get_state() -> Result<DashState, String> {
        if !tauri::present() {
            return Ok(mock_state());
        }
        tauri::invoke("get_state", tauri::none()).await
    }

    pub async fn save_config(config: &Config) -> Result<(), String> {
        if !tauri::present() {
            MOCK.with(|m| *m.borrow_mut() = Some(config.clone()));
            return Ok(());
        }
        tauri::call("save_config", json!({ "config": config })).await
    }

    pub async fn weather_now() -> Result<Option<Reading>, String> {
        if !tauri::present() {
            let code = match dom::param("weather").as_deref() {
                Some("cloudy") => 2,
                Some("overcast") => 3,
                Some("fog") => 45,
                Some("rain") => 63,
                Some("storm") => 95,
                Some("snow") => 73,
                _ => return Ok(None),
            };
            return Ok(Some(Reading { code, cloud: if code == 2 { 55.0 } else { 100.0 }, wind: 15.0, sunrise: 0, sunset: 0 }));
        }
        tauri::invoke("weather_now", tauri::none()).await
    }
}

// ---------- data ----------

async fn load() {
    let Ok(st) = api::get_state().await else { return };
    app(|a| {
        if a.config.is_none() {
            a.config = Some(st.config.clone());
        }
        let rules = &a.config.as_ref().unwrap().rules;
        if !a.selected.as_ref().is_some_and(|s| rules.iter().any(|r| &r.id == s)) {
            a.selected = rules.first().map(|r| r.id.clone());
        }
        a.state = Some(st);
    });
    render_all();
}

fn spawn_load() {
    tauri::spawn(load());
}

fn schedule_save() {
    let old = app(|a| a.save_timer.take());
    dom::clear_timeout(old);
    let t = dom::timeout(450, || tauri::spawn(save()));
    app(|a| a.save_timer = Some(t));
}

async fn save() {
    let Some(config) = app(|a| a.config.clone()) else { return };
    match api::save_config(&config).await {
        Ok(()) => {
            flash("Saved");
            render_list();
            render_today();
        }
        Err(e) => flash(&format!("Couldn't save: {e}")),
    }
}

fn flash(text: &str) {
    let el = dom::by_id("saved");
    el.set_text_content(Some(text));
    dom::set_style(&el, "opacity", "1");
    let old = app(|a| a.flash_timer.take());
    dom::clear_timeout(old);
    let t = dom::timeout(1600, move || dom::set_style(&el, "opacity", "0"));
    app(|a| a.flash_timer = Some(t));
}

// ---------- page ----------

fn switch(id: &str) -> B {
    h("input").attr("type", "checkbox").class("switch").id(id)
}

fn titled(title: &str, small: &str) -> B {
    h("span").child(h("strong").text(title)).child(h("small").text(small))
}

fn number(id: &str, min: u32, max: u32) -> B {
    h("input").attr("type", "number").id(id).attr("min", &min.to_string()).attr("max", &max.to_string())
}

fn unit(input: B, label: &str) -> B {
    h("span").class("unit").child(input).text(label)
}

fn tab_button(tab: &str, label: &str, selected: bool) -> B {
    h("button").attr("role", "tab").attr("data-tab", tab).attr("aria-selected", if selected { "true" } else { "false" }).text(label)
}

fn build() {
    let header = h("header")
        .class("bar")
        .attr("data-tauri-drag-region", "")
        .child(s("svg").class("bar-stars").id("bar-stars").attr("aria-hidden", "true"))
        .child(s("svg").class("treeline").id("treeline").attr("aria-hidden", "true"))
        .child(
            h("nav")
                .class("tabs")
                .attr("role", "tablist")
                .attr("aria-label", "Sections")
                .child(tab_button("rules", "Rules", true))
                .child(tab_button("activity", "Activity", false))
                .child(tab_button("setup", "Setup ", false).child(h("span").class("dot").id("setup-dot").flag("hidden", true))),
        )
        .child(h("span").class("saved").id("saved").attr("aria-live", "polite"));

    let rules = h("section")
        .id("tab-rules")
        .class("tab rules")
        .attr("role", "tabpanel")
        .child(
            h("aside")
                .class("rule-list")
                .child(h("h2").text("Rules"))
                .child(h("ul").id("rule-list"))
                .child(h("button").class("add").id("add-rule").text("New rule"))
                .child(h("p").class("today").id("today-line")),
        )
        .child(h("div").class("editor").id("editor"));

    let activity = h("section").id("tab-activity").class("tab activity").attr("role", "tabpanel").flag("hidden", true).child(
        h("div")
            .class("wrap")
            .child(h("h1").id("activity-headline"))
            .child(h("p").class("lede").id("activity-lede"))
            .child(h("div").class("chart").id("week-chart").attr("aria-label", "Last 7 days"))
            .child(
                h("p")
                    .class("legend")
                    .child(h("span").class("key k-back"))
                    .text("Turned back ")
                    .child(h("span").class("key k-open"))
                    .text("Opened anyway ")
                    .child(h("span").class("key k-other").id("k-other"))
                    .child(h("span").id("k-other-label").text("Left without choosing")),
            )
            .child(
                h("table")
                    .class("table")
                    .id("rule-table")
                    .child(h("thead").child(h("tr").kids(
                        ["Rule", "Pauses", "Turned back", "Opened", "Time today"].map(|t| h("th").text(t)),
                    )))
                    .child(h("tbody")),
            ),
    );

    let seg = |value: &str, label: &str| {
        h("label").child(h("input").attr("type", "radio").attr("name", "sky").attr("value", value)).text(&format!(" {label}"))
    };
    let setup = h("section").id("tab-setup").class("tab setup").attr("role", "tabpanel").flag("hidden", true).child(
        h("div")
            .class("wrap")
            .child(h("h1").text("Setup"))
            .child(
                h("div")
                    .class("group")
                    .child(h("h3").text("Browsers"))
                    .child(h("p").class("help").text("Defixum reads the address of the tab you're looking at. Firefox-based browsers like Zen are read through Accessibility, so no extension is needed. Safari and Chromium browsers are asked directly, and macOS checks with you once per browser."))
                    .child(h("div").class("sheet").child(h("div").class("row").id("ax-row")).child(h("ul").class("browsers").id("browsers"))),
            )
            .child(
                h("div").class("group").child(h("h3").text("Pause screen")).child(
                    h("div")
                        .class("sheet")
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Open-for choices", "Minutes offered after the pause, separated by commas"))
                                .child(h("input").attr("type", "text").id("session-options").attr("inputmode", "numeric").attr("placeholder", "2, 5, 15")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Stop counting time when idle for", "Time on a site only counts while you're using the computer"))
                                .child(unit(number("idle-seconds", 10, 900), " sec")),
                        ),
                ),
            )
            .child(
                h("div").class("group").child(h("h3").text("App switching")).child(
                    h("div")
                        .class("sheet")
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Limit ⌘Tab", "Too many app switches in a minute and ⌘Tab rests for a while. Hold it for 5 seconds to get through"))
                                .child(switch("switch-limit")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Switches allowed", "⌘Tab presses in any one minute before the cooldown starts"))
                                .child(unit(number("switches-per-minute", 1, 60).attr("value", "6"), " a minute")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Cooldown", "How long ⌘Tab stays off once you go over"))
                                .child(unit(number("switch-cooldown", 5, 3600).attr("value", "60"), " sec")),
                        ),
                ),
            )
            .child(
                h("div").class("group").child(h("h3").text("Breaks")).child(
                    h("div")
                        .class("sheet")
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Remind me to move", "A nudge to get up after a long stretch at the screen. During a video or call it's just a note at the top"))
                                .child(switch("move-reminder")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Every", "Three minutes away from the Mac starts the count over"))
                                .child(unit(number("move-every", 15, 180).attr("value", "45"), " min")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Break choices", "Minutes offered on the reminder, separated by commas"))
                                .child(h("input").attr("type", "text").id("move-breaks").attr("inputmode", "numeric").attr("placeholder", "1, 3, 5, 10")),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Reminder stays for", "Then it goes away on its own"))
                                .child(unit(number("move-prompt", 5, 60).attr("value", "10"), " sec")),
                        ),
                ),
            )
            .child(
                h("div").class("group").child(h("h3").text("General")).child(
                    h("div")
                        .class("sheet")
                        .child(
                            h("div")
                                .class("row")
                                .child(titled("Sky", "How the pause screen and this window are lit"))
                                .child(
                                    h("span")
                                        .class("seg")
                                        .attr("role", "radiogroup")
                                        .attr("aria-label", "Sky")
                                        .child(seg("time", "Time of day"))
                                        .child(seg("light", "Light"))
                                        .child(seg("dark", "Dark")),
                                ),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Weather", "Rain, snow, fog and cloud on the pause screen follow the weather outside. Uses your approximate location from your internet address"))
                                .child(switch("weather")),
                        )
                        .child(
                            h("div")
                                .class("row bed")
                                .child(
                                    h("div")
                                        .class("bed-top")
                                        .child(
                                            h("label")
                                                .attr("for", "bedtime")
                                                .child(h("strong").text("Bedtime reminder"))
                                                .child(h("small").text("A gentle full-screen reminder 2 hours, 1 hour, 30, 15 and 5 minutes before, and at bedtime. Past it, you choose when to be reminded again")),
                                        )
                                        .child(
                                            h("span")
                                                .class("unit")
                                                .child(h("input").attr("type", "time").id("bedtime-at").attr("aria-label", "Bedtime").attr("value", "23:00"))
                                                .text(" ")
                                                .child(switch("bedtime")),
                                        ),
                                )
                                .child(
                                    h("div")
                                        .class("bed-more")
                                        .id("bed-more")
                                        .flag("hidden", true)
                                        .child(h("div").class("week").id("bed-week").attr("role", "group").attr("aria-label", "Bedtime by day"))
                                        .child(h("small").id("bed-days-note").text("The same time every night. Pick a night to change it or turn it off")),
                                ),
                        )
                        .child(
                            h("label")
                                .class("row")
                                .child(titled("Open at login", "Defixum lives in the menu bar and needs to be running to pause you"))
                                .child(switch("autostart")),
                        ),
                ),
            ),
    );

    let body = dom::body();
    body.append_child(&header.el()).unwrap();
    body.append_child(&h("main").child(rules).child(activity).child(setup).el()).unwrap();
}

fn hint_ring(dash: &str) -> B {
    s("svg")
        .class("hint")
        .attr("viewBox", "0 0 40 40")
        .attr("aria-hidden", "true")
        .child(s("circle").attr("cx", "20").attr("cy", "20").attr("r", "16").attr("opacity", ".22"))
        .child(s("circle").class("arc").attr("cx", "20").attr("cy", "20").attr("r", "16").attr("pathLength", "100").attr("stroke-dasharray", dash))
}

fn radio_choice(name: &str, value: &str, title: &str, small: &str) -> B {
    h("label")
        .child(h("input").attr("type", "radio").attr("name", name).attr("value", value))
        .child(h("span").child(h("strong").text(title)).child(h("small").text(small)))
}

fn countdown_choice(hint: B, value: &str, title: &str, small: &str) -> B {
    h("label")
        .child(hint)
        .child(
            h("span")
                .class("opt")
                .child(h("input").attr("type", "radio").attr("name", "countdown").attr("value", value))
                .child(h("strong").text(title)),
        )
        .child(h("small").text(small))
}

fn prompt_choice(value: &str, title: &str, small: &str) -> B {
    h("label")
        .child(h("input").attr("type", "checkbox").attr("name", "prompt").attr("value", value))
        .child(h("span").child(h("strong").text(title)).child(h("small").text(small)))
}

fn limit_box(id: &str) -> B {
    h("input").attr("type", "checkbox").id(id)
}

fn editor_dom() -> B {
    let breathe = s("svg")
        .class("hint")
        .attr("viewBox", "0 0 40 40")
        .attr("aria-hidden", "true")
        .child(s("circle").attr("cx", "20").attr("cy", "20").attr("r", "17").attr("opacity", ".25"))
        .child(s("circle").attr("cx", "20").attr("cy", "20").attr("r", "12").attr("opacity", ".55"))
        .child(s("circle").attr("cx", "20").attr("cy", "20").attr("r", "7"));
    h("div")
        .class("editor-inner")
        .child(
            h("div")
                .class("editor-head")
                .child(h("input").class("name").id("f-name").attr("aria-label", "Rule name").attr("placeholder", "Rule name"))
                .child(h("label").class("switch-label").child(switch("f-enabled")).text(" ").child(h("span").id("f-enabled-text").text("On"))),
        )
        .child(
            h("div")
                .class("field")
                .child(h("h3").text("Sites"))
                .child(h("div").class("chips").id("f-sites").child(h("input").id("f-site-input").attr("placeholder", "Add a site, like youtube.com").attr("aria-label", "Add a site")))
                .child(h("p").class("help").text("Subdomains are included. Add a path to narrow it, like reddit.com/r/all.")),
        )
        .child(
            h("div")
                .class("field")
                .child(h("h3").text("When you visit"))
                .child(
                    h("div")
                        .class("choice")
                        .attr("role", "radiogroup")
                        .child(radio_choice("mode", "pause", "Pause first", "Wait, then choose to close the tab or open it for a few minutes"))
                        .child(radio_choice("mode", "block", "Block", "No way through, only the option to close the tab. Useful with a schedule")),
                )
                .child(
                    h("label")
                        .class("inline")
                        .id("pause-len")
                        .text("Pause for ")
                        .child(h("input").attr("type", "range").id("f-pause").attr("min", "3").attr("max", "60").attr("step", "1"))
                        .text(" ")
                        .child(h("output").id("f-pause-out")),
                )
                .child(
                    h("div")
                        .id("pause-style")
                        .child(h("h4").text("Countdown"))
                        .child(
                            h("div")
                                .class("choice thirds")
                                .attr("role", "radiogroup")
                                .attr("aria-label", "Countdown")
                                .child(countdown_choice(breathe, "breathe", "Breathe", "A circle that grows and shrinks with your breath"))
                                .child(countdown_choice(hint_ring("68 100"), "fill", "Fill", "A ring that fills clockwise"))
                                .child(countdown_choice(hint_ring("28 100"), "drain", "Drain", "A ring that empties back to 12")),
                        )
                        .child(h("h4").text("Message"))
                        .child(
                            h("div")
                                .class("choice rows")
                                .child(prompt_choice("reflect", "Reflect", "A mindful question while you wait"))
                                .child(prompt_choice("cost", "Count the cost", "What this visit takes from you. Some only appear late at night"))
                                .child(prompt_choice("commit", "Type to unlock", "Type out what you're choosing, word for word, before the site opens")),
                        )
                        .child(h("p").class("help").id("prompt-help").text("One is picked at random each time.")),
                ),
        )
        .child(
            h("div")
                .class("field")
                .id("limits")
                .child(h("h3").text("Daily limits"))
                .child(
                    h("div")
                        .class("pair")
                        .child(h("label").class("limit").child(limit_box("f-opens-on")).text(" At most ").child(number("f-opens", 1, 99)).text(" opens a day"))
                        .child(h("label").class("limit").child(limit_box("f-mins-on")).text(" At most ").child(number("f-mins", 1, 1440)).text(" minutes a day")),
                )
                .child(h("label").class("limit apart").id("f-apart-row").child(limit_box("f-apart")).text(" Give each site its own limits"))
                .child(h("p").class("help").text("Once a limit is hit, the site stays blocked until tomorrow.")),
        )
        .child(
            h("div")
                .class("field")
                .child(h("h3").text("Schedule"))
                .child(h("label").class("limit").child(limit_box("f-sched-on")).text(" Only on certain days and times"))
                .child(
                    h("div")
                        .class("sched")
                        .id("sched")
                        .child(h("div").class("days").id("f-days"))
                        .child(
                            h("label")
                                .class("inline")
                                .text("From ")
                                .child(h("input").attr("type", "time").id("f-start"))
                                .text(" to ")
                                .child(h("input").attr("type", "time").id("f-end")),
                        ),
                ),
        )
        .child(h("div").class("editor-foot").child(h("button").class("danger").id("f-delete").text("Delete rule")))
}

// ---------- tabs ----------

fn bind_tabs() {
    for b in dom::qa(".tabs button") {
        let me = b.clone();
        dom::on::<Event>(&b, "click", move |_| select_tab(&me));
    }
    // ?tab=activity|setup opens on that tab (design work and screenshots)
    if let Some(start) = dom::param("tab") {
        if let Some(b) = dom::qa(".tabs button").into_iter().find(|b| b.dataset().get("tab").as_deref() == Some(&*start)) {
            b.click();
        }
    }
}

fn select_tab(b: &HtmlElement) {
    let tab = b.dataset().get("tab").unwrap_or_default();
    for x in dom::qa(".tabs button") {
        let _ = x.set_attribute("aria-selected", if &x == b { "true" } else { "false" });
    }
    for t in dom::qa(".tab") {
        t.set_hidden(t.id() != format!("tab-{tab}"));
    }
    if tab != "rules" {
        spawn_load();
    }
}

fn render_all() {
    render_list();
    render_editor();
    render_today();
    render_activity();
    render_setup();
}

// ---------- rule list ----------

fn rule_summary(r: &Rule) -> String {
    let what = if r.mode == "block" { "Blocks".to_string() } else { format!("{}s pause", r.pause_seconds) };
    let sites = if r.sites.is_empty() {
        "No sites yet".to_string()
    } else {
        let mut s = r.sites.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
        if r.sites.len() > 2 {
            s += &format!(" +{}", r.sites.len() - 2);
        }
        s
    };
    format!("{what}, {sites}")
}

fn tree_icon() -> B {
    s("svg")
        .class("state")
        .attr("viewBox", "0 0 11 14")
        .attr("aria-hidden", "true")
        .child(s("path").attr("d", "M5.5 .7 8.9 5.6H7.2l2.9 4.5H6.3v3.2H4.7v-3.2H.9l2.9-4.5H2.1Z"))
}

fn render_list() {
    let Some((rules, selected)) = app(|a| a.config.as_ref().map(|c| (c.rules.clone(), a.selected.clone()))) else { return };
    let ul = dom::by_id("rule-list");
    dom::clear(&ul);
    for r in rules {
        let id = r.id.clone();
        let b = h("button")
            .class(if r.enabled { "" } else { "off" })
            .attr("aria-current", if selected.as_deref() == Some(&r.id) { "true" } else { "false" })
            .child(tree_icon())
            .child(h("span").class("rname").text(if r.name.is_empty() { "Untitled rule" } else { &r.name }))
            .child(h("span").class("rsub").text(&rule_summary(&r)))
            .on::<Event>("click", move |_| {
                app(|a| a.selected = Some(id.clone()));
                render_list();
                render_editor();
            });
        ul.append_child(&h("li").child(b).el()).unwrap();
    }
}

fn bind_add_rule() {
    dom::on::<Event>(&dom::by_id("add-rule"), "click", |_| {
        if app(|a| a.config.is_none()) {
            return;
        }
        let r = Rule { id: format!("r{}", dom::now_ms() as u64), ..Rule::default() };
        let id = r.id.clone();
        cfg(|c| c.rules.push(r));
        let old = app(|a| {
            a.selected = Some(id);
            a.save_timer.take()
        });
        dom::clear_timeout(old);
        tauri::spawn(async {
            save().await;
            render_list();
            render_editor();
            let name = dom::input("f-name");
            let _ = name.focus();
            name.select();
        });
    });
}

fn render_today() {
    let ids = app(|a| a.config.as_ref().map(|c| c.rules.iter().map(|r| r.id.clone()).collect::<Vec<_>>())).unwrap_or_default();
    let (mut pauses, mut back) = (0, 0);
    for id in &ids {
        let s = today_stats(id);
        pauses += s.intercepts;
        back += s.skips;
    }
    let line = if pauses == 0 {
        "No pauses yet today.".to_string()
    } else {
        format!("Today you paused {} and turned back {}.", times(pauses), times(back))
    };
    dom::by_id("today-line").set_text_content(Some(&line));
}

// ---------- editor ----------

fn clean_site(s: &str) -> String {
    let mut s = s.trim().to_lowercase();
    if let Some(i) = s.find("://") {
        if i > 0 && s[..i].bytes().all(|b| b.is_ascii_lowercase()) {
            s = s[i + 3..].to_string();
        }
    }
    if let Some(rest) = s.strip_prefix("www.") {
        s = rest.to_string();
    }
    let s = s.trim_end_matches('/').to_string();
    let host = s.split('/').next().unwrap_or("");
    let labels: Vec<&str> = host.split('.').collect();
    let ok = labels.len() >= 2
        && labels.iter().all(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'));
    if ok {
        s
    } else {
        String::new()
    }
}

fn render_editor() {
    let host = dom::by_id("editor");
    let Some(r) = app(|a| {
        let sel = a.selected.clone()?;
        a.config.as_ref()?.rules.iter().find(|r| r.id == sel).cloned()
    }) else {
        dom::clear(&host);
        host.append_child(&h("p").class("editor-empty").text("Add a rule to choose which sites make you pause.").el()).unwrap();
        return;
    };
    dom::clear(&host);
    host.append_child(&editor_dom().el()).unwrap();
    let id = r.id.clone();

    // name and on/off
    let name = dom::input("f-name");
    name.set_value(&r.name);
    {
        let (id, name2) = (id.clone(), name.clone());
        dom::on::<Event>(&name, "input", move |_| {
            with_rule(&id, |r| r.name = name2.value());
            render_list();
            schedule_save();
        });
    }
    let enabled = dom::input("f-enabled");
    enabled.set_checked(r.enabled);
    dom::by_id("f-enabled-text").set_text_content(Some(if r.enabled { "On" } else { "Off" }));
    {
        let (id, en) = (id.clone(), enabled.clone());
        dom::on::<Event>(&enabled, "change", move |_| {
            let on = en.checked();
            with_rule(&id, |r| r.enabled = on);
            dom::by_id("f-enabled-text").set_text_content(Some(if on { "On" } else { "Off" }));
            render_list();
            schedule_save();
        });
    }

    bind_sites(&id);

    // mode and pause
    for radio in inputs("input[name=\"mode\"]") {
        radio.set_checked(radio.value() == r.mode);
        let (id, rd) = (id.clone(), radio.clone());
        dom::on::<Event>(&radio, "change", move |_| {
            let mode = rd.value();
            with_rule(&id, |r| r.mode = mode);
            sync_mode(&id);
            render_list();
            schedule_save();
        });
    }
    let pause = dom::input("f-pause");
    pause.set_value(&r.pause_seconds.to_string());
    let draw_pause = |p: &HtmlInputElement| dom::by_id("f-pause-out").set_text_content(Some(&format!("{} seconds", p.value())));
    draw_pause(&pause);
    {
        let (id, p) = (id.clone(), pause.clone());
        dom::on::<Event>(&pause, "input", move |_| {
            let v = num(&p.value()) as u32;
            with_rule(&id, |r| r.pause_seconds = v);
            draw_pause(&p);
            render_list();
            schedule_save();
        });
    }
    sync_mode(&id);

    // countdown style
    let countdown = if r.countdown.is_empty() { "breathe".to_string() } else { r.countdown.clone() };
    for radio in inputs("input[name=\"countdown\"]") {
        radio.set_checked(radio.value() == countdown);
        let (id, rd) = (id.clone(), radio.clone());
        dom::on::<Event>(&radio, "change", move |_| {
            let v = rd.value();
            with_rule(&id, |r| r.countdown = v);
            schedule_save();
        });
    }

    // message kinds: at least one stays checked
    let kinds = if r.prompts.is_empty() { vec!["reflect".to_string()] } else { r.prompts.clone() };
    let boxes = inputs("input[name=\"prompt\"]");
    let draw_help = |boxes: &[HtmlInputElement]| dom::by_id("prompt-help").set_hidden(boxes.iter().filter(|b| b.checked()).count() < 2);
    for b in &boxes {
        b.set_checked(kinds.contains(&b.value()));
        let (id, all, me) = (id.clone(), boxes.clone(), b.clone());
        dom::on::<Event>(b, "change", move |_| {
            if !all.iter().any(|b| b.checked()) {
                me.set_checked(true);
            }
            let chosen: Vec<String> = all.iter().filter(|b| b.checked()).map(|b| b.value()).collect();
            with_rule(&id, |r| r.prompts = chosen);
            draw_help(&all);
            schedule_save();
        });
    }
    draw_help(&boxes);

    // limits
    bind_limit(&id, "f-opens-on", "f-opens", r.daily_opens, 5.0, |r, v| r.daily_opens = v);
    bind_limit(&id, "f-mins-on", "f-mins", r.daily_minutes, 30.0, |r, v| r.daily_minutes = v);
    let apart = dom::input("f-apart");
    apart.set_checked(r.separate_limits);
    let draw_apart = {
        let id = id.clone();
        move || {
            let none = with_rule(&id, |r| r.daily_opens.is_none() && r.daily_minutes.is_none()).unwrap_or(true);
            dom::by_id("f-apart-row").set_hidden(none);
        }
    };
    draw_apart();
    {
        let (id, ap) = (id.clone(), apart.clone());
        dom::on::<Event>(&apart, "change", move |_| {
            let on = ap.checked();
            with_rule(&id, |r| r.separate_limits = on);
            schedule_save();
        });
    }
    for box_id in ["f-opens-on", "f-mins-on"] {
        let d = draw_apart.clone();
        dom::on::<Event>(&dom::by_id(box_id), "change", move |_| d());
    }

    // schedule
    let s_on = dom::input("f-sched-on");
    s_on.set_checked(r.schedule.enabled);
    let draw_sched = |on: bool| dom::toggle_class(&dom::by_id("sched"), "off", !on);
    draw_sched(r.schedule.enabled);
    {
        let (id, so) = (id.clone(), s_on.clone());
        dom::on::<Event>(&s_on, "change", move |_| {
            let on = so.checked();
            with_rule(&id, |r| r.schedule.enabled = on);
            draw_sched(on);
            schedule_save();
        });
    }
    let days = dom::by_id("f-days");
    for (i, d) in DAY_NAMES.iter().enumerate() {
        let b = h("button").text(d).attr("aria-pressed", if r.schedule.days[i] { "true" } else { "false" }).html();
        let (id, me) = (id.clone(), b.clone());
        dom::on::<Event>(&b, "click", move |_| {
            let now = with_rule(&id, |r| {
                r.schedule.days[i] = !r.schedule.days[i];
                r.schedule.days[i]
            })
            .unwrap_or(false);
            let _ = me.set_attribute("aria-pressed", if now { "true" } else { "false" });
            schedule_save();
        });
        days.append_child(&b).unwrap();
    }
    for (el_id, start) in [("f-start", true), ("f-end", false)] {
        let el = dom::input(el_id);
        el.set_value(if start { &r.schedule.start } else { &r.schedule.end });
        let (id, e2) = (id.clone(), el.clone());
        dom::on::<Event>(&el, "change", move |_| {
            let v = e2.value();
            if v.is_empty() {
                return;
            }
            with_rule(&id, |r| if start { r.schedule.start = v } else { r.schedule.end = v });
            schedule_save();
        });
    }

    // Two-step delete: window.confirm() is a silent no-op in the app's web view.
    let del = dom::by_id("f-delete");
    let timer: std::rc::Rc<std::cell::Cell<Option<i32>>> = Default::default();
    {
        let (id, d) = (id.clone(), del.clone());
        dom::on::<Event>(&del, "click", move |_| {
            if !dom::has_class(&d, "armed") {
                let name = with_rule(&id, |r| r.name.clone()).unwrap_or_default();
                let _ = d.class_list().add_1("armed");
                d.set_text_content(Some(&format!("Click again to delete “{}”", if name.is_empty() { "this rule" } else { &name })));
                dom::clear_timeout(timer.take());
                let d2 = d.clone();
                timer.set(Some(dom::timeout(4000, move || {
                    let _ = d2.class_list().remove_1("armed");
                    d2.set_text_content(Some("Delete rule"));
                })));
                return;
            }
            dom::clear_timeout(timer.take());
            let id = id.clone();
            app(|a| {
                if let Some(c) = a.config.as_mut() {
                    c.rules.retain(|r| r.id != id);
                    a.selected = c.rules.first().map(|r| r.id.clone());
                }
            });
            render_list();
            render_editor();
            schedule_save();
        });
    }
}

fn sync_mode(id: &str) {
    let block = with_rule(id, |r| r.mode == "block").unwrap_or(false);
    for el in ["pause-len", "pause-style", "limits"] {
        dom::by_id(el).set_hidden(block);
    }
}

fn bind_limit(id: &str, check_id: &str, num_id: &str, value: Option<u32>, fallback: f64, set: fn(&mut Rule, Option<u32>)) {
    let check = dom::input(check_id);
    let n = dom::input(num_id);
    check.set_checked(value.is_some());
    n.set_value(&value.map(|v| v.to_string()).unwrap_or_else(|| fallback.to_string()));
    n.set_disabled(!check.checked());
    {
        let (id, c, n) = (id.to_string(), check.clone(), n.clone());
        dom::on::<Event>(&check, "change", move |_| {
            n.set_disabled(!c.checked());
            let v = c.checked().then(|| num_or(&n.value(), fallback).max(1.0).round() as u32);
            with_rule(&id, |r| set(r, v));
            schedule_save();
        });
    }
    {
        let (id, c, n2) = (id.to_string(), check.clone(), n.clone());
        dom::on::<Event>(&n, "input", move |_| {
            let v = num(&n2.value());
            if c.checked() && v > 0.0 {
                with_rule(&id, |r| set(r, Some(v.round() as u32)));
                schedule_save();
            }
        });
    }
}

fn bind_sites(id: &str) {
    let chips = dom::by_id("f-sites");
    let input = dom::input("f-site-input");

    fn draw(id: &str) {
        let chips = dom::by_id("f-sites");
        let input = dom::input("f-site-input");
        for c in dom::qa_in(&chips, ".chip") {
            c.remove();
        }
        let sites = with_rule(id, |r| r.sites.clone()).unwrap_or_default();
        for (i, site) in sites.iter().enumerate() {
            let id = id.to_string();
            let x = h("button")
                .attr("aria-label", &format!("Remove {site}"))
                .child(s("svg").attr("viewBox", "0 0 8 8").attr("aria-hidden", "true").child(s("path").attr("d", "M1 1l6 6M7 1 1 7")))
                .on::<MouseEvent>("click", move |e| {
                    e.stop_propagation();
                    with_rule(&id, |r| {
                        if i < r.sites.len() {
                            r.sites.remove(i);
                        }
                    });
                    draw(&id);
                    render_list();
                    schedule_save();
                });
            let chip = h("span").class("chip").text(site).child(x).el();
            chips.insert_before(&chip, Some(&input)).unwrap();
        }
    }

    fn add(id: &str, text: &str) {
        let added = with_rule(id, |r| {
            let mut added = false;
            for s in text.split(|c: char| c.is_whitespace() || c == ',').map(clean_site) {
                if !s.is_empty() && !r.sites.contains(&s) {
                    r.sites.push(s);
                    added = true;
                }
            }
            added
        })
        .unwrap_or(false);
        if added {
            draw(id);
            render_list();
            schedule_save();
        }
    }

    draw(id);
    {
        let inp = input.clone();
        dom::on::<Event>(&chips, "click", move |_| {
            let _ = inp.focus();
        });
    }
    {
        let (id, inp) = (id.to_string(), input.clone());
        dom::on::<KeyboardEvent>(&input, "keydown", move |e| {
            let key = e.key();
            let value = inp.value();
            if (key == "Enter" || key == "," || key == " ") && !value.trim().is_empty() {
                e.prevent_default();
                add(&id, &value);
                inp.set_value("");
            } else if key == "Backspace" && value.is_empty() {
                let popped = with_rule(&id, |r| r.sites.pop().is_some()).unwrap_or(false);
                if popped {
                    draw(&id);
                    render_list();
                    schedule_save();
                }
            }
        });
    }
    {
        let (id, inp) = (id.to_string(), input.clone());
        dom::on::<Event>(&input, "blur", move |_| {
            let value = inp.value();
            if !value.trim().is_empty() {
                add(&id, &value);
                inp.set_value("");
            }
        });
    }
    {
        let id = id.to_string();
        dom::on::<ClipboardEvent>(&input, "paste", move |e| {
            e.prevent_default();
            let text = e.clipboard_data().and_then(|d| d.get_data("text").ok()).unwrap_or_default();
            add(&id, &text);
        });
    }
}

// ---------- activity ----------

struct Day {
    key: String,
    label: String,
    today: bool,
    sum: DayStats,
}

fn render_activity() {
    let Some((state, rules)) = app(|a| Some((a.state.clone()?, a.config.as_ref()?.rules.clone()))) else { return };
    let base = js_sys::Date::new(&format!("{}T12:00:00", state.today).into());
    let mut days = vec![];
    for i in (0..7).rev() {
        let d = js_sys::Date::new_with_year_month_day(base.get_full_year(), base.get_month() as i32, base.get_date() as i32 - i);
        let key = format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date());
        let mut sum = DayStats::default();
        if let Some(rs) = state.stats.get(&key) {
            for s in rs.values() {
                sum.intercepts += s.intercepts;
                sum.opens += s.opens;
                sum.skips += s.skips;
                sum.seconds += s.seconds;
            }
        }
        // a pause that started before midnight and ended after it can leave a day with more choices than pauses
        sum.intercepts = sum.intercepts.max(sum.skips + sum.opens);
        let label = if i == 0 { "Today".to_string() } else { DAY_NAMES[((d.get_day() + 6) % 7) as usize].to_string() };
        days.push(Day { key, label, today: i == 0, sum });
    }
    let t = days[6].sum;
    let (wi, ws, wsec) = days.iter().fold((0u32, 0u32, 0u64), |a, d| (a.0 + d.sum.intercepts, a.1 + d.sum.skips, a.2 + d.sum.seconds));
    dom::by_id("activity-headline").set_text_content(Some(&if t.intercepts == 0 {
        "Nothing has needed a pause today.".to_string()
    } else {
        format!("You paused {} today and turned back {}.", times(t.intercepts), times(t.skips))
    }));
    dom::by_id("activity-lede").set_text_content(Some(&if wi == 0 {
        "Once a rule catches a site, each pause shows up here.".to_string()
    } else {
        format!(
            "Over the last 7 days: {wi} pauses, {}% of them ended with the tab closed, and {} spent on paused sites.",
            (ws as f64 / wi as f64 * 100.0).round(),
            fmt_minutes(wsec)
        )
    }));

    // Bars total every pause; the ones that ended without a choice (switched away) get their own segment.
    let other = |d: &DayStats| d.intercepts - d.skips - d.opens;
    let any_other = days.iter().any(|d| other(&d.sum) > 0);
    dom::by_id("k-other").set_hidden(!any_other);
    dom::by_id("k-other-label").set_hidden(!any_other);
    let max = days.iter().map(|d| d.sum.intercepts).max().unwrap_or(0).max(1);
    let chart = dom::by_id("week-chart");
    dom::clear(&chart);
    for d in &days {
        let s = d.sum;
        let col = h("div")
            .class(if d.today { "col is-today" } else { "col" })
            .attr(
                "title",
                &format!("{}: {} turned back, {} opened, {} left without choosing, {}", d.key, s.skips, s.opens, other(&s), fmt_minutes(s.seconds)),
            )
            .child(h("span").class("n").text(&if s.intercepts > 0 { s.intercepts.to_string() } else { String::new() }))
            .child(
                h("div")
                    .class("stack")
                    .attr("style", &format!("height:{}px", forest::num(s.intercepts as f64 / max as f64 * 140.0 + 2.0)))
                    .child(h("div").class("back").attr("style", &format!("flex:{}", s.skips)))
                    .child(h("div").class("open").attr("style", &format!("flex:{}", s.opens)))
                    .child(h("div").class("other").attr("style", &format!("flex:{}", other(&s)))),
            )
            .child(h("span").class("d").text(&d.label));
        chart.append_child(&col.el()).unwrap();
    }

    let tbody = dom::q_in(&dom::by_id("rule-table"), "tbody");
    dom::clear(&tbody);
    if rules.is_empty() {
        tbody.append_child(&h("tr").child(h("td").attr("colspan", "5").class("empty").text("No rules yet.")).el()).unwrap();
        return;
    }
    for r in &rules {
        let s = today_stats(&r.id);
        let mins = s.seconds / 60;
        let over = r.daily_minutes.is_some_and(|m| m > 0 && mins >= m as u64);
        let opens_over = r.daily_opens.is_some_and(|o| o > 0 && s.opens >= o);
        let opens = match r.daily_opens.filter(|&o| o > 0) {
            Some(o) => format!("{} of {o}", s.opens),
            None => s.opens.to_string(),
        };
        let time = match r.daily_minutes.filter(|&m| m > 0) {
            Some(m) => format!("{} of {m} min", fmt_minutes(s.seconds)),
            None => fmt_minutes(s.seconds),
        };
        let tr = h("tr")
            .child(h("td").text(if r.name.is_empty() { "Untitled rule" } else { &r.name }))
            .child(h("td").text(&s.intercepts.to_string()))
            .child(h("td").text(&s.skips.to_string()))
            .child(h("td").class(if opens_over { "over" } else { "" }).text(&opens))
            .child(h("td").class(if over { "over" } else { "" }).text(&time));
        tbody.append_child(&tr.el()).unwrap();
    }
}

// ---------- setup ----------

fn btn(text: &str, f: impl FnMut(Event) + 'static) -> B {
    h("button").class("btn").text(text).on::<Event>("click", f)
}

fn open_privacy(pane: &'static str) {
    tauri::spawn(async move {
        let _ = tauri::call("open_privacy", json!({ "pane": pane })).await;
    });
}

fn render_setup() {
    let Some(state) = app(|a| a.state.clone()) else { return };
    let set = settings();
    let needs_ax = state.browsers.iter().any(|b| b.kind == "ax");
    let ax = dom::by_id("ax-row");
    ax.set_class_name(if needs_ax && !state.accessibility { "row warn" } else { "row" });
    dom::clear(&ax);
    if state.accessibility {
        ax.append_child(&titled("Accessibility is allowed", "Used to read the address in Zen, Firefox and similar browsers").el()).unwrap();
        ax.append_child(&h("span").class("status ok").text("Allowed").el()).unwrap();
    } else {
        ax.append_child(&titled("Allow Accessibility", "Needed for Zen, Firefox and similar browsers. Turn on Defixum in the list that opens").el()).unwrap();
        ax.append_child(&btn("Open System Settings", |_| open_privacy("accessibility")).id("ax-open").el()).unwrap();
    }

    let ul = dom::by_id("browsers");
    dom::clear(&ul);
    if state.browsers.is_empty() {
        ul.append_child(&h("li").child(h("span").class("bname").text("No supported browsers found")).el()).unwrap();
    } else {
        for b in &state.browsers {
            ul.append_child(&browser_row(b, state.accessibility).el()).unwrap();
        }
    }

    let problem = (needs_ax && !state.accessibility) || state.browsers.iter().any(|b| b.status.as_deref() == Some("denied"));
    dom::by_id("setup-dot").set_hidden(!problem);

    let so = dom::input("session-options");
    if !dom::is_active(&so) {
        so.set_value(&join_options(&set.session_options));
    }
    let idle = dom::input("idle-seconds");
    if !dom::is_active(&idle) {
        idle.set_value(&set.idle_seconds.to_string());
    }
    dom::input("autostart").set_checked(state.autostart);
    let sky = if set.sky.is_empty() { "time" } else { &set.sky };
    if let Some(r) = inputs("input[name=\"sky\"]").into_iter().find(|r| r.value() == sky) {
        r.set_checked(true);
    }
    dom::input("weather").set_checked(set.weather);
    dom::input("bedtime").set_checked(set.bedtime);
    let at = dom::input("bedtime-at");
    if !dom::is_active(&at) {
        at.set_value(if set.bedtime_at.is_empty() { "23:00" } else { &set.bedtime_at });
    }
    at.set_disabled(!set.bedtime);
    draw_bedtime();
    dom::input("switch-limit").set_checked(set.switch_limit);
    let per = dom::input("switches-per-minute");
    if !dom::is_active(&per) {
        per.set_value(&(if set.switches_per_minute == 0 { 6 } else { set.switches_per_minute }).to_string());
    }
    per.set_disabled(!set.switch_limit);
    let cool = dom::input("switch-cooldown");
    if !dom::is_active(&cool) {
        cool.set_value(&(if set.switch_cooldown_seconds == 0 { 60 } else { set.switch_cooldown_seconds }).to_string());
    }
    cool.set_disabled(!set.switch_limit);
    dom::input("move-reminder").set_checked(set.move_reminder);
    for (id, v) in [("move-every", set.move_every_minutes), ("move-prompt", set.move_prompt_seconds)] {
        let el = dom::input(id);
        if !dom::is_active(&el) {
            el.set_value(&v.to_string());
        }
        el.set_disabled(!set.move_reminder);
    }
    let mb = dom::input("move-breaks");
    if !dom::is_active(&mb) {
        mb.set_value(&join_options(&set.move_breaks));
    }
    mb.set_disabled(!set.move_reminder);
    apply_sky();
}

fn join_options(v: &[u32]) -> String {
    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

fn browser_row(b: &BrowserInfo, accessibility: bool) -> B {
    let how = if b.kind == "ax" { "through Accessibility" } else { "through Apple Events" };
    let name = h("span").class("bname").text(&b.name).child(h("small").text(how));
    let ok = if b.kind == "ax" { accessibility } else { b.status.as_deref() == Some("ok") };
    let (status, class, button): (String, &str, Option<B>) = if ok {
        ("Working".into(), "status ok", None)
    } else if b.status.as_deref() == Some("denied") {
        let pane = if b.kind == "ax" { "accessibility" } else { "automation" };
        ("Not allowed".into(), "status bad", Some(btn("Open System Settings", move |_| open_privacy(pane))))
    } else if b.kind != "ax" {
        let text = if b.running { "Not checked yet".to_string() } else { format!("Open {} to check", b.name) };
        let check = b.running.then(|| {
            let (bundle, bname) = (b.bundle.clone(), b.name.clone());
            btn("Check", move |_| {
                let (bundle, bname) = (bundle.clone(), bname.clone());
                tauri::spawn(async move {
                    let r: Result<Option<String>, String> = tauri::invoke("test_browser", json!({ "bundle": bundle })).await;
                    if matches!(r.as_ref().map(|s| s.as_deref()), Ok(Some("not_running"))) {
                        flash(&format!("Open {bname} first"));
                    }
                    load().await;
                });
            })
        });
        (text, "status", check)
    } else {
        ("Needs Accessibility".into(), "status bad", None)
    };
    let li = h("li").child(name).child(h("span").class(class).text(&status));
    match button {
        Some(b) => li.child(b),
        None => li,
    }
}

fn bind_setup() {
    let so = dom::input("session-options");
    {
        let e = so.clone();
        dom::on::<Event>(&so, "change", move |_| {
            let mut list: Vec<u32> = e
                .value()
                .split(|c: char| c.is_whitespace() || c == ',')
                .map(num)
                .filter(|n| n.fract() == 0.0 && (1.0..=240.0).contains(n))
                .map(|n| n as u32)
                .collect();
            let out = cfg(|c| {
                if !list.is_empty() {
                    list.sort_unstable();
                    list.dedup();
                    list.truncate(4);
                    c.settings.session_options = list;
                }
                join_options(&c.settings.session_options)
            });
            if let Some(v) = out {
                e.set_value(&v);
            }
            schedule_save();
        });
    }
    bind_number("idle-seconds", 10.0, 900.0, 60.0, |s, v| s.idle_seconds = v as u64);
    for r in inputs("input[name=\"sky\"]") {
        let me = r.clone();
        dom::on::<Event>(&r, "change", move |_| {
            let v = me.value();
            cfg(|c| c.settings.sky = v);
            apply_sky();
            schedule_save();
        });
    }
    let weather = dom::input("weather");
    {
        let w = weather.clone();
        dom::on::<Event>(&weather, "change", move |_| {
            let on = w.checked();
            cfg(|c| c.settings.weather = on);
            schedule_save();
            // the app fetches a reading within half a minute of the setting going on
            if !on {
                app(|a| a.outside = None);
                apply_sky();
            } else {
                for ms in [8000, 35000] {
                    dom::timeout(ms, || tauri::spawn(refresh_outside()));
                }
            }
        });
    }
    let bedtime = dom::input("bedtime");
    {
        let b = bedtime.clone();
        dom::on::<Event>(&bedtime, "change", move |_| {
            let on = b.checked();
            cfg(|c| c.settings.bedtime = on);
            dom::input("bedtime-at").set_disabled(!on);
            if !on {
                app(|a| a.bed_day = None);
            }
            draw_bedtime();
            schedule_save();
        });
    }
    let at = dom::input("bedtime-at");
    {
        let a = at.clone();
        dom::on::<Event>(&at, "change", move |_| {
            let v = a.value();
            if !v.is_empty() {
                cfg(|c| c.settings.bedtime_at = v);
            } else {
                a.set_value(&bed_usual());
            }
            draw_bedtime();
            schedule_save();
        });
    }
    let limit = dom::input("switch-limit");
    {
        let l = limit.clone();
        dom::on::<Event>(&limit, "change", move |_| {
            let on = l.checked();
            cfg(|c| c.settings.switch_limit = on);
            dom::input("switches-per-minute").set_disabled(!on);
            dom::input("switch-cooldown").set_disabled(!on);
            schedule_save();
        });
    }
    bind_number("switches-per-minute", 1.0, 60.0, 6.0, |s, v| s.switches_per_minute = v as u32);
    bind_number("switch-cooldown", 5.0, 3600.0, 60.0, |s, v| s.switch_cooldown_seconds = v as u32);
    let mv = dom::input("move-reminder");
    {
        let m = mv.clone();
        dom::on::<Event>(&mv, "change", move |_| {
            let on = m.checked();
            cfg(|c| c.settings.move_reminder = on);
            for id in ["move-every", "move-breaks", "move-prompt"] {
                dom::input(id).set_disabled(!on);
            }
            schedule_save();
        });
    }
    bind_number("move-every", 15.0, 180.0, 45.0, |s, v| s.move_every_minutes = v as u32);
    bind_number("move-prompt", 5.0, 60.0, 10.0, |s, v| s.move_prompt_seconds = v as u32);
    let mb = dom::input("move-breaks");
    {
        let e = mb.clone();
        dom::on::<Event>(&mb, "change", move |_| {
            let mut list: Vec<u32> = e
                .value()
                .split(|c: char| c.is_whitespace() || c == ',')
                .map(num)
                .filter(|n| n.fract() == 0.0 && (1.0..=60.0).contains(n))
                .map(|n| n as u32)
                .collect();
            let out = cfg(|c| {
                if !list.is_empty() {
                    list.sort_unstable();
                    list.dedup();
                    list.truncate(4);
                    c.settings.move_breaks = list;
                }
                join_options(&c.settings.move_breaks)
            });
            if let Some(v) = out {
                e.set_value(&v);
            }
            schedule_save();
        });
    }
    let auto = dom::input("autostart");
    {
        let a = auto.clone();
        dom::on::<Event>(&auto, "change", move |_| {
            let (a, want) = (a.clone(), a.checked());
            tauri::spawn(async move {
                match tauri::invoke::<bool>("set_autostart", json!({ "enabled": want })).await {
                    Ok(v) => a.set_checked(v),
                    Err(e) => {
                        flash(&format!("Couldn't change: {e}"));
                        a.set_checked(!a.checked());
                    }
                }
            });
        });
    }
}

/// A number field kept within `min..=max`, `fallback` when it's empty.
fn bind_number(id: &str, min: f64, max: f64, fallback: f64, set: fn(&mut Settings, f64)) {
    let el = dom::input(id);
    let e = el.clone();
    dom::on::<Event>(&el, "change", move |_| {
        let v = num_or(&e.value(), fallback).round().clamp(min, max);
        cfg(|c| set(&mut c.settings, v));
        e.set_value(&v.to_string());
        schedule_save();
    });
}

// ---------- bedtime by day ----------
// Seven cells, each showing that night's time. The picked one grows to hold its own switch and
// time; with the switch off the time goes and the night is skipped.

fn bed_days() -> Vec<String> {
    let s = settings();
    (0..7).map(|i| s.bedtime_by_day.get(i).cloned().unwrap_or_default()).collect()
}

fn bed_usual() -> String {
    let s = settings();
    if s.bedtime_at.is_empty() {
        "23:00".into()
    } else {
        s.bedtime_at
    }
}

fn set_bed_day(i: usize, value: String) {
    let mut days = bed_days();
    days[i] = value;
    cfg(|c| c.settings.bedtime_by_day = days);
    draw_bedtime();
    schedule_save();
}

fn build_bed_week() {
    let week = dom::by_id("bed-week");
    dom::clear(&week);
    for (i, name) in DAY_NAMES.iter().enumerate() {
        let full = FULL_DAYS[i];
        let sw = h("input").attr("type", "checkbox").class("switch").attr("aria-label", &format!("Remind on {full} nights")).html();
        let at = h("input").attr("type", "time").attr("aria-label", &format!("Bedtime on {full} nights")).html();
        let cell = h("div")
            .class("cell")
            .child(h("button").attr("type", "button").class("face").child(h("span").text(name)).child(h("span").class("t")))
            .child(
                h("div")
                    .class("ctl")
                    .child(h("button").attr("type", "button").class("nm").attr("aria-label", &format!("Close {full}")).text(name))
                    .child(h("span").class("controls").child(sw.clone()).child(at.clone())),
            )
            .html();
        // empty space closes it too
        dom::on::<MouseEvent>(&cell, "click", move |e| {
            let in_input = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.closest("input").ok().flatten().is_some());
            if !in_input {
                app(|a| a.bed_day = if a.bed_day == Some(i) { None } else { Some(i) });
                draw_bedtime();
            }
        });
        let (sw_in, at_in) = (input_el(&sw), input_el(&at));
        {
            let (sw2, at2) = (sw_in.clone(), at_in.clone());
            dom::on::<Event>(&sw, "change", move |_| {
                let v = if sw2.checked() {
                    let t = at2.value();
                    if !t.is_empty() && t != bed_usual() {
                        t
                    } else {
                        String::new()
                    }
                } else {
                    "off".into()
                };
                set_bed_day(i, v);
            });
        }
        {
            let at2 = at_in.clone();
            dom::on::<Event>(&at, "change", move |_| {
                let t = at2.value();
                if t.is_empty() {
                    let d = bed_days()[i].clone();
                    at2.set_value(&if d.is_empty() { bed_usual() } else { d });
                    return;
                }
                set_bed_day(i, if t == bed_usual() { String::new() } else { t });
            });
        }
        week.append_child(&cell).unwrap();
    }
}

fn draw_bedtime() {
    let on = settings().bedtime;
    let usual = bed_usual();
    let days = bed_days();
    dom::by_id("bed-more").set_hidden(!on);
    if !on {
        app(|a| a.bed_day = None);
    }
    let week = dom::by_id("bed-week");
    if week.child_element_count() == 0 {
        build_bed_week();
    }
    let picked_day = app(|a| a.bed_day);
    let cells = dom::qa_in(&week, ".cell");
    for (i, cell) in cells.iter().enumerate() {
        let v = &days[i];
        let off = v == "off";
        let picked = picked_day == Some(i);
        dom::toggle_class(cell, "sel", picked);
        if !picked {
            let _ = cell.style().remove_property("flex-basis");
        }
        dom::toggle_class(cell, "custom", !v.is_empty() && !off);
        dom::toggle_class(cell, "off", off);
        let shown = if off { "Off" } else if v.is_empty() { &usual } else { v };
        let face = dom::q_in(cell, ".face");
        dom::q_in(&face, ".t").set_text_content(Some(shown));
        let _ = face.set_attribute("aria-label", &format!("{} night: {}", FULL_DAYS[i], if off { "no reminder" } else { shown }));
        dom::q_in(cell, ".nm").set_hidden(false);
        input_el(&dom::q_in(cell, ".switch")).set_checked(!off);
        let at = input_el(&dom::q_in(cell, "input[type=time]"));
        if !dom::is_active(&at) {
            at.set_value(if off || v.is_empty() { &usual } else { v });
        }
    }
    if let Some(sel) = picked_day.and_then(|i| cells.get(i)) {
        // hug the toggle and time, whatever the font makes of them
        let ctl = dom::q_in(sel, ".ctl");
        let kids = ctl.children();
        let w = (0..kids.length()).filter_map(|k| kids.item(k)).map(|c| c.get_bounding_client_rect().width()).fold(f64::NEG_INFINITY, f64::max);
        dom::set_style(sel, "flex-basis", &format!("{}px", w.ceil() + 24.0));
    }
    let custom = days.iter().filter(|d| !d.is_empty()).count();
    dom::by_id("bed-days-note").set_text_content(Some(&if custom == 0 {
        "The same time every night. Pick a night to change it or turn it off".to_string()
    } else {
        format!("{custom} {} from the usual time", if custom == 1 { "night differs" } else { "nights differ" })
    }));
}

// ---------- tree line and sky ----------

/// The strip of forest along the bottom of the title bar, low in the middle under the tabs.
/// A new stretch of forest each time the window loads; resizing redraws the same one.
fn draw_treeline() {
    let seed = app(|a| a.bar_seed);
    let svg = dom::by_id("treeline");
    let (w, hgt) = (svg.client_width() as f64, svg.client_height() as f64);
    if w == 0.0 {
        return;
    }
    let _ = svg.set_attribute("viewBox", &format!("0 0 {w} {hgt}"));
    let far = forest::treeline(&forest::Treeline {
        width: w,
        height: hgt,
        base: hgt - 6.0,
        min_h: 4.0,
        max_h: 15.0,
        min_w: 5.0,
        max_w: 10.0,
        seed: seed.wrapping_add(3),
        hills: 2.5,
        spacing: (0.5, 1.1),
        ..Default::default()
    });
    let near = forest::treeline(&forest::Treeline {
        width: w,
        height: hgt,
        base: hgt - 1.0,
        min_h: 3.0,
        max_h: 26.0,
        min_w: 8.0,
        max_w: 17.0,
        seed: seed.wrapping_add(5),
        clearing: Some(forest::Clearing { center: w / 2.0, half: w * 0.3, floor: 0.2 }),
        hills: 1.0,
        ..Default::default()
    });
    dom::clear(&svg);
    svg.append_child(&s("path").class("far").attr("d", &far).el()).unwrap();
    svg.append_child(&s("path").class("near").attr("d", &near).el()).unwrap();

    // stars for the dark sky, kept clear of the tabs in the middle
    let bar = dom::by_id("bar-stars");
    let (bw, bh) = (bar.client_width() as f64, bar.client_height() as f64);
    let mut r = forest::Rng::new(seed.wrapping_add(7));
    let _ = bar.set_attribute("viewBox", &format!("0 0 {bw} {bh}"));
    dom::clear(&bar);
    for _ in 0..(bw / 22.0).round() as u32 {
        let x = r.next() * bw;
        let y = 4.0 + r.next() * (bh - 30.0);
        if (x - bw / 2.0).abs() < 150.0 {
            continue;
        }
        let star = s("circle")
            .attr("cx", &format!("{x:.1}"))
            .attr("cy", &format!("{y:.1}"))
            .attr("r", &format!("{:.2}", 0.4 + r.next() * 0.7))
            .attr("opacity", &format!("{:.2}", 0.3 + r.next() * 0.7));
        bar.append_child(&star.el()).unwrap();
    }
}

async fn refresh_outside() {
    let r = api::weather_now().await.ok().flatten();
    app(|a| a.outside = r);
    apply_sky();
}

/// Sky setting: "time" lights the window like the sky outside right now (dark at night, the title
/// bar tinted for the hour and greyed by cloud); "light" and "dark" fix it.
fn apply_sky() {
    let mode = app(|a| a.config.as_ref().map(|c| c.settings.sky.clone())).filter(|s| !s.is_empty()).unwrap_or_else(|| "time".into());
    let outside = app(|a| a.outside.clone());
    let root = dom::root();
    let theme = if mode == "time" {
        let s = forest::sky(sky::solar_hour(sky::now_hour(), outside.as_ref()), &sky::weather(outside.as_ref()));
        dom::set_var(&root, "--sky-top", &s.top);
        dom::set_var(&root, "--sky-bottom", &s.horizon);
        dom::set_var(&root, "--tree-far", &forest::mix(&s.horizon, &s.near, if s.day { 0.36 } else { 0.5 }));
        dom::set_var(&root, "--tree-near", &forest::mix(&s.horizon, &s.near, if s.day { 0.82 } else { 1.0 }));
        if s.day { "light" } else { "dark" }.to_string()
    } else {
        for v in ["--sky-top", "--sky-bottom", "--tree-far", "--tree-near"] {
            let _ = root.style().remove_property(v);
        }
        mode
    };
    let _ = root.set_attribute("data-theme", &theme);
    if let Ok(Some(ls)) = dom::win().local_storage() {
        let _ = ls.set_item("defixum-theme", &theme);
    }
}

/// The last theme used, so the window opens in it before the settings load.
fn early_theme() {
    let saved = dom::win().local_storage().ok().flatten().and_then(|ls| ls.get_item("defixum-theme").ok().flatten());
    let theme = saved.unwrap_or_else(|| {
        let dark = dom::win().match_media("(prefers-color-scheme: dark)").ok().flatten().is_some_and(|m| m.matches());
        if dark { "dark" } else { "light" }.into()
    });
    let _ = dom::root().set_attribute("data-theme", &theme);
}

// ---------- boot ----------

pub fn start() {
    early_theme();
    dom::add_style(include_str!("../styles/settings.css"));
    app(|a| a.bar_seed = (dom::random() * 2f64.powi(31)).floor() as u32);
    build();
    bind_tabs();
    bind_add_rule();
    bind_setup();

    dom::interval(60_000, || tauri::spawn(refresh_outside()));
    tauri::spawn(refresh_outside());
    draw_treeline();
    dom::on::<Event>(&dom::win(), "resize", |_| {
        let old = app(|a| a.treeline_timer.take());
        dom::clear_timeout(old);
        let t = dom::timeout(120, draw_treeline);
        app(|a| a.treeline_timer = Some(t));
    });

    tauri::listen::<serde_json::Value>("refresh", |_| spawn_load());
    // Cmd+W hides the dashboard, like its close button (the app menu has no Close Window item).
    dom::on::<KeyboardEvent>(&dom::doc(), "keydown", |e| {
        if e.meta_key() && !e.ctrl_key() && !e.alt_key() && !e.shift_key() && e.key().to_lowercase() == "w" {
            e.prevent_default();
            tauri::spawn(async {
                let _ = tauri::call("close_main", tauri::none()).await;
            });
        }
    });
    dom::on::<Event>(&dom::win(), "focus", |_| {
        if app(|a| a.state.is_none()) {
            return;
        }
        tauri::spawn(async {
            if let Ok(s) = api::get_state().await {
                app(|a| a.state = Some(s));
                render_today();
                render_activity();
                render_setup();
            }
        });
    });
    spawn_load();
}
