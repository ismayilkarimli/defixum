//! The pause screen and the bedtime reminder: a clearing at the edge of a forest under the sky
//! you would see right now. The bedtime reminder is the same page in its own window.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::json;
use wasm_bindgen::JsCast;
use web_sys::{
    AnimationEvent, CanvasRenderingContext2d, Element, Event, HtmlCanvasElement, HtmlElement, HtmlTextAreaElement, InputEvent,
    KeyboardEvent,
};

use crate::dom::{self, h, s, B};
use crate::sky::{self, Reading};
use crate::tauri;
use crate::types::{Bedtime, Intercept, Prompt};

enum Cur {
    Pause(Intercept),
    Bed(Bedtime),
}

impl Cur {
    fn sky(&self) -> &str {
        match self {
            Cur::Pause(i) => &i.sky,
            Cur::Bed(b) => &b.sky,
        }
    }
}

/// What the typing box needs to know about the pause on screen.
#[derive(Default, Clone)]
struct Unlock {
    prompt: Option<Prompt>,
    typing: bool,
    draft_key: Option<String>,
    shortest: Option<u32>,
}

#[derive(Clone, Copy)]
struct Drop {
    z: f64,
    x: f64,
    y: f64,
    v: f64,
    len: f64,
    sway: f64,
    sway_w: f64,
}

#[derive(Default)]
struct Ov {
    bedtime_mode: bool,
    current: Option<Cur>,
    unlock: Unlock,
    /// What was typed into the sentence box. The window is only hidden between pauses, so when
    /// the user comes back to a pause they already sat through, the half-typed sentence is still here.
    draft: Option<(String, String)>,
    timer: Option<i32>,
    raf: Option<i32>,
    scene_seed: u32,
    built_for: String,
    weather_for: String,
    weather_now: Option<Reading>,
    drops: Vec<Drop>,
    precip_raf: Option<i32>,
    life_timers: Vec<i32>,
    bedtime_timer: Option<i32>,
}

thread_local! {
    static OV: RefCell<Ov> = RefCell::new(Ov::default());
}

fn ov<R>(f: impl FnOnce(&mut Ov) -> R) -> R {
    OV.with(|o| f(&mut o.borrow_mut()))
}

fn new_seed() -> u32 {
    (dom::random() * 2f64.powi(31)).floor() as u32
}

fn fx(v: f64, d: usize) -> String {
    format!("{v:.d$}")
}

fn typed() -> HtmlTextAreaElement {
    dom::by_id("typed").unchecked_into()
}

fn set_error(text: &str) {
    dom::by_id("error").set_text_content(Some(text));
}

fn invoke_or_error(cmd: &'static str, args: serde_json::Value) {
    tauri::spawn(async move {
        if let Err(e) = tauri::call(cmd, args).await {
            set_error(&e);
        }
    });
}

// ---------- page ----------

fn build() {
    let scene = h("div")
        .class("scene")
        .id("scene")
        .attr("aria-hidden", "true")
        .child(h("div").class("milky").id("milky"))
        .child(s("svg").class("stars").id("stars").attr("preserveAspectRatio", "none"))
        .child(h("div").class("clouds").id("clouds"))
        .child(h("div").class("birds").id("birds"))
        .child(h("canvas").class("precip").id("precip").flag("hidden", true));
    let main = h("main")
        .id("main")
        .attr("aria-live", "polite")
        .child(
            h("div")
                .class("breath")
                .id("breath")
                .child(h("div").class("halo"))
                .child(h("div").class("core"))
                .child(
                    s("svg")
                        .class("ring")
                        .attr("viewBox", "0 0 100 100")
                        .attr("aria-hidden", "true")
                        .child(s("circle").class("track").attr("cx", "50").attr("cy", "50").attr("r", "48.4"))
                        .child(s("circle").class("arc").id("arc").attr("cx", "50").attr("cy", "50").attr("r", "48.4").attr("pathLength", "100")),
                )
                .child(h("div").class("readout").child(h("div").class("count").id("count")).child(h("div").class("cue").id("cue"))),
        )
        .child(h("h1").id("title"))
        .child(
            h("div")
                .class("commit")
                .id("commit")
                .flag("hidden", true)
                .child(h("p").class("sentence").id("sentence").flag("hidden", true))
                .child(
                    h("textarea")
                        .id("typed")
                        .attr("rows", "1")
                        .flag("disabled", true)
                        .attr("spellcheck", "false")
                        .attr("autocomplete", "off")
                        .attr("autocapitalize", "off")
                        .attr("autocorrect", "off")
                        .attr("aria-label", "Your answer")
                        .attr("aria-describedby", "unlock-hint"),
                )
                .child(h("p").class("unlock-hint").id("unlock-hint").attr("aria-live", "polite")),
        )
        .child(h("p").class("detail").id("detail"));
    let floor = h("footer")
        .class("floor")
        .child(
            h("div")
                .class("actions")
                .child(h("button").class("primary").id("back").text("Close the tab"))
                .child(h("div").class("later").id("later").child(h("span").text("or open it for")))
                .child(h("div").class("later").id("mute-row").flag("hidden", true).child(h("button").id("mute").text("Don't remind again today"))),
        )
        .child(h("p").class("error").id("error").attr("role", "alert"))
        .child(h("p").class("hint").text("Press Esc or ⌘W to close the tab"));
    let body = dom::body();
    for b in [scene, main, floor] {
        body.append_child(&b.el()).unwrap();
    }
}

// ---------- the pause ----------

fn mins(m: u64) -> String {
    format!("{m} minute{}", if m == 1 { "" } else { "s" })
}

fn detail_text(i: &Intercept) -> String {
    let mut parts = vec![];
    let opens = i.opens_today;
    let daily_opens = i.daily_opens.filter(|&n| n > 0);
    let daily_minutes = i.daily_minutes.filter(|&n| n > 0);
    match i.reason.as_str() {
        "pause" | "expired" => {
            if let Some(d) = daily_opens {
                let left = d.saturating_sub(opens);
                parts.push(if left == 1 { "This is your last open for today.".to_string() } else { format!("You have {left} of {d} opens left today.") });
            } else if opens > 0 {
                parts.push(format!("You've opened it {} today.", if opens == 1 { "once".to_string() } else { format!("{opens} times") }));
            } else {
                parts.push("You haven't opened it yet today.".into());
            }
            if let Some(d) = daily_minutes {
                parts.push(format!("{} of {d} minutes used.", i.minutes_today));
            } else if i.minutes_today > 0 {
                parts.push(format!("{} so far.", mins(i.minutes_today)));
            }
        }
        "open_limit" => parts.push(format!("You've used all {} opens for today. It opens again tomorrow.", i.daily_opens.unwrap_or(0))),
        "time_limit" => parts.push(format!("You've spent your {} for today. It opens again tomorrow.", mins(i.daily_minutes.unwrap_or(0) as u64))),
        "blocked" => parts.push(format!("Your “{}” rule is keeping it closed.", i.rule_name)),
        _ => {}
    }
    parts.join(" ")
}

fn site_span(site: &str) -> Element {
    h("span").class("site").text(if site.is_empty() { "This site" } else { site }).el()
}

fn set_title(before: &str, site: &str, after: &str) {
    let t = dom::by_id("title");
    dom::clear(&t);
    let _ = t.append_with_str_1(before);
    t.append_child(&site_span(site)).unwrap();
    let _ = t.append_with_str_1(after);
}

/// The headline or sentence with the site picked out in the glow colour.
fn render_with_site(el: &HtmlElement, text: &str, site: &str) {
    dom::clear(el);
    let parts: Vec<&str> = if site.is_empty() { vec![text] } else { text.split(site).collect() };
    for (n, part) in parts.iter().enumerate() {
        if n > 0 {
            el.append_child(&site_span(site)).unwrap();
        }
        if !part.is_empty() {
            let _ = el.append_with_str_1(part);
        }
    }
}

/// Keep in step with prompts::unlocks on the Rust side, which has the final say.
fn normalize(s: &str) -> String {
    let s: String = s
        .trim()
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            c => c,
        })
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let s = s.strip_suffix('.').unwrap_or(&s);
    s.trim_end().to_string()
}

fn unlocks(p: Option<&Prompt>, typed: &str) -> bool {
    match p.and_then(|p| p.sentence.as_deref()) {
        Some(sentence) => normalize(typed) == normalize(sentence),
        None => true,
    }
}

/// What's still missing, in words, so a disabled button never goes unexplained.
fn unlock_hint(p: Option<&Prompt>, typed: &str, shortest: Option<u32>) -> (bool, String) {
    if unlocks(p, typed) {
        let m = shortest.map(|m| m.to_string()).unwrap_or_else(|| "undefined".into());
        return (true, format!("Press Return to open it for {m} min, or pick a time below."));
    }
    if typed.trim().is_empty() {
        return (false, "Type the sentence exactly as shown.".into());
    }
    let sentence = p.and_then(|p| p.sentence.as_deref()).unwrap_or("");
    let on_track = normalize(sentence).starts_with(&normalize(typed));
    (false, if on_track { "Keep going." } else { "Not quite. Check for a typo." }.into())
}

fn autosize(el: &HtmlTextAreaElement) {
    let style = el.style();
    let _ = style.set_property("height", "auto");
    let _ = style.set_property("height", &format!("{}px", (el.scroll_height() + 2).min(110)));
}

fn sync_unlock() {
    let u = ov(|o| o.unlock.clone());
    let t = typed();
    let ok = unlocks(u.prompt.as_ref(), &t.value());
    dom::toggle_class(&t, "ok", u.typing && ok);
    for b in dom::qa_in(&dom::by_id("later"), "button") {
        let _ = b.toggle_attribute_with_force("disabled", !ok);
    }
    let hint = dom::by_id("unlock-hint");
    if u.typing && !t.disabled() {
        let (ready, text) = unlock_hint(u.prompt.as_ref(), &t.value(), u.shortest);
        hint.set_text_content(Some(&text));
        dom::toggle_class(&hint, "ready", ready);
    } else {
        hint.set_text_content(Some(""));
        dom::toggle_class(&hint, "ready", false);
    }
}

fn stop_countdown() {
    let (t, r) = ov(|o| (o.timer.take(), o.raf.take()));
    dom::clear_interval(t);
    dom::cancel_frame(r);
}

fn show(i: Intercept) {
    let body = dom::body();
    // each new pause walks into a different stretch of forest; a re-render keeps the one in view
    if !dom::has_class(&body, "in") {
        ov(|o| o.scene_seed = new_seed());
    }
    ov(|o| {
        o.weather_now = i.weather.clone();
        o.current = Some(Cur::Pause(i.clone()));
    });
    paint();
    start_life();
    stop_countdown();
    set_error("");
    let main = dom::by_id("main");
    let breath = dom::by_id("breath");
    let can_continue = i.reason == "pause" || i.reason == "expired";
    let prompt = if can_continue { i.prompt.clone() } else { None };
    let style = if can_continue && !i.countdown.is_empty() { i.countdown.clone() } else { "breathe".to_string() };
    let ring = style == "fill" || style == "drain";
    let sentence = prompt.as_ref().and_then(|p| p.sentence.clone());
    let typing = sentence.is_some();
    let draft_key = sentence.as_ref().map(|s| format!("{}\n{s}", i.rule_id));
    if !i.waited {
        ov(|o| o.draft = None);
    }
    breath.set_class_name(if ring { "breath is-ring" } else { "breath" });
    let _ = body.class_list().remove_2("waiting", "breathing");
    dom::toggle_class(&main, "typing", typing);
    ov(|o| o.unlock = Unlock { prompt: prompt.clone(), typing, draft_key: draft_key.clone(), shortest: i.session_options.first().copied() });

    // type to unlock
    let t = typed();
    let draft = ov(|o| o.draft.clone());
    t.set_value(match (&draft, &draft_key) {
        (Some((k, text)), Some(key)) if i.waited && k == key => text,
        _ => "",
    });
    t.set_disabled(true);
    dom::toggle_class(&t, "ok", false);
    t.set_placeholder("You can type once the countdown ends");
    let _ = t.style().set_property("height", "");
    let _ = t.style().set_property("min-height", "");
    dom::by_id("commit").set_hidden(!typing);
    dom::by_id("sentence").set_hidden(sentence.is_none());
    if let Some(sentence) = &sentence {
        render_with_site(&dom::by_id("sentence"), sentence, &i.site);
        // Size the box for the whole sentence up front, so typing it never makes the box grow.
        let text = t.value();
        t.set_value(sentence);
        autosize(&t);
        let height = t.style().get_property_value("height").unwrap_or_default();
        let _ = t.style().set_property("min-height", &height);
        t.set_value(&text);
        let _ = t.style().set_property("height", "");
    }

    // continue buttons
    let later = dom::by_id("later");
    for b in dom::qa_in(&later, "button") {
        b.remove();
    }
    later.set_hidden(!can_continue);
    if can_continue {
        for &m in &i.session_options {
            let b = h("button").text(&format!("{m} min")).on::<Event>("click", move |_| cont(m));
            later.append_child(&b.el()).unwrap();
        }
        sync_unlock();
    }

    let expired = if prompt.is_some() && i.reason == "expired" { "Your time is up. " } else { "" };
    dom::by_id("detail").set_text_content(Some(&format!("{expired}{}", detail_text(&i))));

    if !can_continue {
        let _ = breath.class_list().add_1("stopped");
        dom::by_id("count").set_text_content(Some(""));
        dom::by_id("cue").set_text_content(Some(""));
        if i.reason == "blocked" {
            let until = match &i.until {
                Some(u) if !u.is_empty() => format!("until {u}"),
                _ => "right now".into(),
            };
            set_title("", &i.site, &format!(" is off {until}."));
        } else {
            set_title("That's enough ", &i.site, " for today.");
        }
        let _ = dom::by_id("back").focus();
        dom::frame(|_| {
            let _ = dom::body().class_list().add_1("in");
        });
        return;
    }

    // the pause
    let _ = body.class_list().add_1("waiting");
    if let Some(p) = &prompt {
        render_with_site(&dom::by_id("title"), &p.text, &i.site);
    } else if i.reason == "expired" {
        set_title("Your time on ", &i.site, " is up.");
    } else {
        set_title("", &i.site, " can wait a few seconds.");
    }
    let run = Rc::new(Run {
        started: dom::perf_now(),
        total: i.pause_seconds as f64,
        fill: style == "fill",
        reduce_motion: dom::reduce_motion(),
        intercept: i.clone(),
        has_prompt: prompt.is_some(),
        typing,
        shown_left: std::cell::Cell::new(-1),
    });

    if i.waited {
        // the countdown was already served: arrive in the state it ends in
        if ring {
            run.draw(1.0);
        }
        run.finish();
        autosize(&t);
    } else if ring {
        ring_frame(run);
        let _ = dom::by_id("back").focus();
    } else {
        let _ = breath.class_list().add_1("breathing");
        let _ = body.class_list().add_1("breathing");
        let r2 = run.clone();
        let tick = move || {
            let elapsed = (dom::perf_now() - r2.started) / 1000.0;
            let left = (r2.total - elapsed).ceil().max(0.0);
            dom::by_id("count").set_text_content(Some(&if left > 0.0 { forest::num(left) } else { String::new() }));
            let cue = if left > 0.0 {
                if elapsed % 8.0 < 4.0 {
                    "Breathe in"
                } else {
                    "Breathe out"
                }
            } else {
                ""
            };
            dom::by_id("cue").set_text_content(Some(cue));
            if left == 0.0 {
                dom::clear_interval(ov(|o| o.timer.take()));
                r2.finish();
            }
            left == 0.0
        };
        if !tick() {
            let h = dom::interval(200, move || {
                tick();
            });
            ov(|o| o.timer = Some(h));
        }
        let _ = dom::by_id("back").focus();
    }
    dom::frame(|_| {
        let _ = dom::body().class_list().add_1("in");
    });
}

/// One countdown in progress.
struct Run {
    started: f64,
    total: f64,
    fill: bool,
    reduce_motion: bool,
    intercept: Intercept,
    has_prompt: bool,
    typing: bool,
    shown_left: std::cell::Cell<i64>,
}

impl Run {
    /// fill grows from nothing to a full circle; drain is the same arc retracting to 12 o'clock
    fn draw(&self, p: f64) {
        let len = (if self.fill { p } else { 1.0 - p }) * 100.0;
        let arc: HtmlElement = dom::by_id("arc");
        dom::set_style(&arc, "stroke-dasharray", &format!("{} 100", forest::num(len)));
        dom::set_style(&arc, "visibility", if len < 0.2 { "hidden" } else { "visible" });
    }

    fn finish(&self) {
        let breath = dom::by_id("breath");
        let _ = breath.class_list().remove_1("breathing");
        let _ = breath.class_list().add_1("settled");
        let _ = dom::body().class_list().remove_2("waiting", "breathing");
        dom::by_id("count").set_text_content(Some(""));
        dom::by_id("cue").set_text_content(Some(""));
        let i = &self.intercept;
        if !self.has_prompt {
            if i.reason == "expired" {
                set_title("Your time on ", &i.site, " is up. Another round?");
            } else {
                set_title("Still want ", &i.site, "?");
            }
        }
        if self.typing {
            let t = typed();
            t.set_disabled(false);
            t.set_placeholder("Type the sentence above");
            sync_unlock();
            let _ = t.focus();
        } else {
            let _ = dom::by_id("back").focus();
        }
    }
}

fn ring_frame(run: Rc<Run>) {
    let elapsed = (dom::perf_now() - run.started) / 1000.0;
    let left = (run.total - elapsed).ceil().max(0.0) as i64;
    if left != run.shown_left.get() {
        run.shown_left.set(left);
        dom::by_id("count").set_text_content(Some(&if left > 0 { left.to_string() } else { String::new() }));
    }
    if left == 0 {
        run.draw(1.0);
        run.finish();
        return;
    }
    let shown = if run.reduce_motion { elapsed.floor() } else { elapsed };
    run.draw((shown / run.total).min(1.0));
    let h = dom::frame(move |_| ring_frame(run));
    ov(|o| o.raf = Some(h));
}

fn cont(m: u32) {
    let typed = if dom::by_id("commit").hidden() { serde_json::Value::Null } else { typed().value().into() };
    invoke_or_error("overlay_continue", json!({ "minutes": m, "typed": typed }));
}

fn back() {
    invoke_or_error("overlay_back", tauri::none());
}

// ---------- the forest ----------
// Five tree lines from the far ridge to the giants framing the clearing. Each pause draws a new
// forest (scene_seed); a resize redraws the same one. Colours follow the Sky setting.

struct Layer {
    /// fraction of the screen height; None stands on the forest floor
    base: Option<f64>,
    min_h: f64,
    max_h: f64,
    min_w: f64,
    max_w: f64,
    tone: f64,
    mist: Option<f64>,
    hills: f64,
    spacing: (f64, f64),
    /// how low trees get mid-screen
    clear: f64,
    half: f64,
    scrub: f64,
}

const SPACING: (f64, f64) = (0.32, 0.72);

const LAYERS: [Layer; 5] = [
    Layer { base: Some(0.66), min_h: 0.035, max_h: 0.12, min_w: 0.018, max_w: 0.036, tone: 0.28, mist: Some(120.0), hills: 0.045, spacing: (0.5, 1.15), clear: 0.55, half: 0.28, scrub: 0.0 },
    Layer { base: Some(0.72), min_h: 0.045, max_h: 0.16, min_w: 0.026, max_w: 0.05, tone: 0.48, mist: Some(95.0), hills: 0.03, spacing: (0.45, 1.0), clear: 0.5, half: 0.3, scrub: 0.0 },
    Layer { base: Some(0.78), min_h: 0.06, max_h: 0.2, min_w: 0.038, max_w: 0.075, tone: 0.67, mist: Some(75.0), hills: 0.018, spacing: SPACING, clear: 0.7, half: 0.36, scrub: 0.0 },
    Layer { base: None, min_h: 0.05, max_h: 0.28, min_w: 0.06, max_w: 0.115, tone: 0.86, mist: None, hills: 0.014, spacing: SPACING, clear: 0.2, half: 0.36, scrub: 0.02 },
    Layer { base: Some(1.0), min_h: 0.0, max_h: 0.7, min_w: 0.12, max_w: 0.2, tone: 1.0, mist: None, hills: 0.0, spacing: SPACING, clear: 0.0, half: 0.5, scrub: 0.0 },
];

fn screen() -> (f64, f64) {
    let w = dom::win();
    (
        w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0),
        w.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0),
    )
}

fn px(v: f64) -> String {
    format!("{}px", forest::num(v))
}

fn build_scene() {
    let (w, hgt) = screen();
    let seed = ov(|o| o.scene_seed);
    let key = format!("{w}x{hgt}:{seed}");
    if ov(|o| o.built_for == key) {
        return;
    }
    ov(|o| o.built_for = key);
    let scene = dom::by_id("scene");
    for n in dom::qa_in(&scene, ".layer, .mists, .glimmers") {
        n.remove();
    }
    let mut r = forest::Rng::new(seed);
    // a little variety in how tall and dense this stretch of forest is
    let tall = 0.85 + r.next() * 0.3;
    let main_bottom = dom::win()
        .get_computed_style(&dom::by_id("main"))
        .ok()
        .flatten()
        .and_then(|cs| cs.get_property_value("bottom").ok())
        .and_then(|b| b.trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(0.0);
    let floor_top = hgt - main_bottom;
    for (i, l) in LAYERS.iter().enumerate() {
        let base = match l.base {
            None => floor_top + 6.0,
            Some(b) => b * hgt + if b == 1.0 { 4.0 } else { 0.0 },
        };
        let d = forest::treeline(&forest::Treeline {
            width: w,
            height: hgt,
            base,
            min_h: l.min_h * hgt,
            max_h: l.max_h * hgt * if i < 3 { tall } else { 1.0 },
            min_w: l.min_w * hgt,
            max_w: l.max_w * hgt,
            seed: seed.wrapping_add(i as u32 * 131),
            clearing: Some(forest::Clearing { center: w / 2.0, half: w * l.half, floor: l.clear }),
            hills: l.hills * hgt,
            scrub: l.scrub * hgt,
            spacing: l.spacing,
        });
        let svg = s("svg")
            .class("layer")
            .attr("viewBox", &format!("0 0 {w} {hgt}"))
            .attr("preserveAspectRatio", "none")
            .var("--i", &i.to_string())
            .var("--depth", &(LAYERS.len() - i).to_string())
            .var("--fill", &format!("var(--l{})", i + 1))
            .child(s("path").attr("d", &d));
        scene.append_child(&svg.el()).unwrap();
        if let Some(mist) = l.mist {
            // fog sits in front of this tree line and behind the nearer ones
            let m = h("div").class("mist").html();
            dom::set_style(&m, "top", &px(base - hgt * 0.075));
            dom::set_style(&m, "height", &px(hgt * 0.13));
            dom::set_var(&m, "--drift", &format!("{}s", forest::num(mist)));
            scene.append_child(&h("div").class("mists").child(m).el()).unwrap();
        }
        // fireflies and golden motes hover in front of the third tree line, behind the near ones
        if i == 2 {
            for kind in ["fireflies", "motes"] {
                let motes = kind == "motes";
                let g = h("div").class(&format!("glimmers {kind}")).el();
                let n = if motes { 16 + (r.next() * 10.0).floor() as u32 } else { 12 + (r.next() * 10.0).floor() as u32 };
                for _ in 0..n {
                    let b = h("b").html();
                    // mostly beside the clearing
                    let side = if r.next() < 0.5 { r.next() * 0.3 } else { 0.7 + r.next() * 0.3 };
                    let left = (if motes { r.next() } else { side }) * w;
                    dom::set_style(&b, "left", &px(left));
                    let up = if motes { 0.05 + r.next() * 0.35 } else { r.next() * 0.16 };
                    dom::set_style(&b, "top", &px(floor_top - hgt * up));
                    let size = if motes { 1.5 + r.next() * 2.0 } else { 2.0 + r.next() * 2.0 };
                    dom::set_var(&b, "--s", &format!("{}px", fx(size, 1)));
                    dom::set_var(&b, "--t", &format!("{}s", fx(9.0 + r.next() * 12.0, 1)));
                    dom::set_var(&b, "--g", &format!("{}s", fx(2.5 + r.next() * 4.0, 1)));
                    dom::set_var(&b, "--d", &format!("{}s", fx(-r.next() * 20.0, 1)));
                    for (x, y) in [("--x1", "--y1"), ("--x2", "--y2"), ("--x3", "--y3")] {
                        let wob = (r.next() - 0.5) * if motes { 60.0 } else { 90.0 };
                        dom::set_var(&b, x, &format!("{}px", fx(wob, 0)));
                        let rise = if motes { -20.0 - r.next() * 50.0 } else { (r.next() - 0.5) * 50.0 };
                        dom::set_var(&b, y, &format!("{}px", fx(rise, 0)));
                    }
                    g.append_child(&b).unwrap();
                }
                scene.append_child(&g).unwrap();
            }
        }
    }
    scene.append_child(&dom::by_id("precip")).unwrap();

    // stars, only visible at night; a share of them gather along the Milky Way, and some twinkle
    let stars = dom::by_id("stars");
    let sh = hgt * 0.6;
    let tilt = -8.0 - r.next() * 22.0;
    let my = 0.05 + r.next() * 0.25;
    let milky = dom::by_id("milky");
    dom::set_var(&milky, "--milky-tilt", &format!("{}deg", fx(tilt, 1)));
    dom::set_var(&milky, "--milky-y", &format!("{}%", fx(my * 100.0 - 23.0, 1)));
    let slope = (tilt * std::f64::consts::PI / 180.0).tan();
    let _ = stars.set_attribute("viewBox", &format!("0 0 {w} {sh}"));
    dom::clear(&stars);
    let count = (w / 9.0).round() as i64;
    for k in (0..count).rev() {
        let on_band = k % 3 == 0;
        let x = r.next() * w;
        let y = if on_band {
            hgt * (my + 0.05) + (x - w / 2.0) * slope + (r.next() + r.next() + r.next() - 1.5) * hgt * 0.07
        } else {
            r.next().powf(1.3) * sh
        };
        if y < 0.0 || y > sh {
            continue;
        }
        let big = r.next() < 0.06;
        let radius = if big {
            1.1 + r.next() * 0.6
        } else if on_band {
            0.3 + r.next() * 0.5
        } else {
            0.4 + r.next() * 0.8
        };
        let opacity = if on_band { 0.2 + r.next() * 0.5 } else { 0.3 + r.next() * 0.7 };
        let mut c = s("circle").attr("cx", &fx(x, 1)).attr("cy", &fx(y, 1)).attr("r", &fx(radius, 2)).attr("opacity", &fx(opacity, 2));
        if r.next() < 0.35 {
            let tw = 1.6 + r.next() * 3.4;
            let twd = -r.next() * 5.0;
            c = c.class("tw").var("--tw", &format!("{}s", fx(tw, 1))).var("--twd", &format!("{}s", fx(twd, 1)));
        }
        stars.append_child(&c.el()).unwrap();
    }
}

// ---------- weather ----------

/// The weather outside, from the app (when the Weather setting is on and it has a recent
/// reading), or from `?weather=` for design work.
fn weather_reading() -> Option<Reading> {
    let demo = match dom::param("weather").as_deref() {
        Some("clear") => Some((0, 5.0)),
        Some("cloudy") => Some((2, 55.0)),
        Some("overcast") => Some((3, 100.0)),
        Some("fog") => Some((45, 100.0)),
        Some("drizzle") => Some((53, 100.0)),
        Some("rain") => Some((63, 100.0)),
        Some("downpour") => Some((65, 100.0)),
        Some("storm") => Some((95, 100.0)),
        Some("snow") => Some((73, 100.0)),
        _ => None,
    };
    match demo {
        Some((code, cloud)) => {
            let wind = dom::param("wind").filter(|w| !w.is_empty()).and_then(|w| w.parse().ok()).unwrap_or(12.0);
            Some(Reading { code, cloud, wind, sunrise: 0, sunset: 0 })
        }
        None => ov(|o| o.weather_now.clone()),
    }
}

/// The sky's weather, and whether it comes from a real reading.
fn wx() -> (forest::Weather, bool) {
    let r = weather_reading();
    (sky::weather(r.as_ref()), r.is_some())
}

/// The hour whose sky to show: now (in this Mac's time zone), or fixed for Light and Dark.
fn sky_hour() -> f64 {
    if let Some(h) = dom::param("hour") {
        return if h.is_empty() { 0.0 } else { h.parse().unwrap_or(f64::NAN) };
    }
    let mode = ov(|o| o.current.as_ref().map(|c| c.sky().to_string()))
        .filter(|s| !s.is_empty())
        .or_else(|| dom::param("sky").filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "time".into());
    match mode.as_str() {
        "light" => 11.0,
        "dark" => 23.5,
        // the clock, stretched so dawn and dusk land on the real sunrise and sunset outside
        _ => sky::solar_hour(sky::now_hour(), weather_reading().as_ref()),
    }
}

/// Colours the sky and tree lines for this hour. True when the weather changed what's drawn.
fn paint() -> bool {
    let hour = sky_hour();
    let (w, _) = wx();
    let s = forest::sky(hour, &w);
    let root = dom::root();
    let set = |k: &str, v: &str| dom::set_var(&root, k, v);
    set("--golden", &fx(forest::golden(hour) * (1.0 - w.gloom), 2));
    set("--sky-top", &s.top);
    set("--horizon", &s.horizon);
    set("--near", &s.near);
    set("--stars", &forest::num(s.stars));
    // fog, and to a lesser degree rain and snow, swallow the far tree lines
    let thick = 1.0 + w.fog * 2.2 + w.rain * 0.5 + w.snow * 0.7;
    for (i, l) in LAYERS.iter().enumerate() {
        set(&format!("--l{}", i + 1), &forest::mix(&s.horizon, &s.near, l.tone.powf(thick)));
    }
    dom::toggle_class(&root, "night", !s.day);
    // clouds are bright by day; after sunset they keep only a little of the last light
    let lit = forest::mix(&s.horizon, "#ffffff", if s.day { 0.7 } else { 0.18 });
    set("--cloud-base", &forest::mix(&lit, if s.day { "#5d6766" } else { "#1c2424" }, w.gloom * 0.55));
    set("--gloom", &fx(w.gloom, 2));
    set("--deck", &fx(((w.cover - 0.55) / 0.45).max(0.0), 2));
    set("--dry", if w.rain > 0.0 || w.snow > 0.0 { "0" } else { "1" });
    set("--cloud-dim", if s.day { "1" } else { "0.55" });
    set(
        "--scrim",
        &if s.day { format!("rgba(250, 252, 244, {})", fx(s.haze, 2)) } else { format!("rgba(4, 12, 10, {})", fx(s.haze, 2)) },
    );
    build_scene();
    build_weather()
}

/// Clouds, rain and snow. Without a weather reading a pause gets a few clouds (sometimes a clear
/// sky); with one, the count, size and speed follow the cloud cover and the wind.
fn build_weather() -> bool {
    let (w, hgt) = screen();
    let (wx, known) = wx();
    let seed = ov(|o| o.scene_seed);
    let key = format!("{w}x{hgt}:{seed}:{}:{}:{}:{}:{}", wx.kind, fx(wx.cover, 1), fx(wx.wind, 1), wx.rain, wx.snow);
    if ov(|o| o.weather_for == key) {
        return false;
    }
    ov(|o| o.weather_for = key);
    build_clouds(w, hgt, &mut forest::Rng::new(seed.wrapping_add(7)), &wx, known);
    start_precip();
    true
}

fn build_clouds(w: f64, hgt: f64, r: &mut forest::Rng, wx: &forest::Weather, known: bool) {
    let bx = dom::by_id("clouds");
    dom::clear(&bx);
    bx.append_child(&h("div").class("deck").el()).unwrap();
    let count = if !known {
        if r.next() < 0.18 {
            0
        } else {
            1 + (r.next() * 4.0).floor() as u32
        }
    } else if wx.cover < 0.08 {
        0
    } else {
        (1.0 + wx.cover * 8.0 + r.next() * 2.0).round() as u32
    };
    for _ in 0..count {
        let w0 = w * (0.16 + r.next() * 0.2) * (1.0 + wx.cover * 0.6);
        let c = h("div").class("cloud").html();
        let t = (260.0 + r.next() * 260.0) / (1.0 + wx.wind * 3.0);
        dom::set_style(&c, "top", &px(hgt * (0.03 + r.next() * (0.3 + wx.cover * 0.08))));
        dom::set_var(&c, "--w", &format!("{}px", fx(w0, 0)));
        dom::set_var(&c, "--t", &format!("{}s", fx(t, 0)));
        dom::set_var(&c, "--d", &format!("{}s", fx(-r.next() * t, 0)));
        dom::set_var(&c, "--a", &format!("{}%", fx((38.0 + r.next() * 30.0 + wx.cover * 25.0).min(92.0), 0)));
        dom::set_var(&c, "--rest", &format!("{}px", fx(r.next() * w, 0)));
        // flat-bottomed puffs: a wide base and smaller rounded heads on top
        let puffs = 4 + (r.next() * 4.0).floor() as u32;
        for p in 0..puffs {
            let pw = if p == 0 { 1.0 } else { 0.3 + r.next() * 0.35 };
            let ph = if p == 0 { 0.55 } else { pw * (1.3 + r.next() * 0.8) };
            let left = if p == 0 { 0.0 } else { 0.08 + r.next() * (0.84 - pw) };
            let i = h("i").html();
            dom::set_style(&i, "left", &format!("{}%", forest::num(left * 100.0)));
            dom::set_style(&i, "width", &format!("{}%", forest::num(pw * 100.0)));
            dom::set_style(&i, "height", &format!("{}%", forest::num(ph * 100.0)));
            let bottom = if p == 0 { 0.0 } else { 12.0 + r.next() * 18.0 };
            dom::set_style(&i, "bottom", &format!("{}%", forest::num(bottom)));
            c.append_child(&i).unwrap();
        }
        bx.append_child(&c).unwrap();
    }
}

// Rain streaks or snowflakes on one canvas, each at its own depth: nearer ones are larger,
// faster and brighter. With reduced motion they are drawn once and stay still.

fn stop_precip() {
    dom::cancel_frame(ov(|o| o.precip_raf.take()));
}

struct Precip {
    ctx: CanvasRenderingContext2d,
    w: f64,
    h: f64,
    snow: bool,
    slant: f64,
    drift: f64,
    fade: f64,
}

impl Precip {
    fn spawn(&self, anywhere: bool) -> Drop {
        let rnd = dom::random;
        let z = 0.25 + rnd() * 0.75;
        Drop {
            z,
            x: rnd() * (self.w + self.slant * self.h) - self.slant * self.h,
            y: if anywhere { rnd() * self.h } else { -30.0 - rnd() * 60.0 },
            v: if self.snow { 22.0 + 46.0 * z } else { 760.0 + 560.0 * z },
            len: 12.0 + 22.0 * z,
            sway: rnd() * 6.3,
            sway_w: 0.4 + rnd() * 0.9,
        }
    }

    fn draw(&self, t: f64) {
        let ctx = &self.ctx;
        ctx.clear_rect(0.0, 0.0, self.w, self.h);
        ov(|o| {
            if self.snow {
                ctx.set_fill_style_str("#f3f6f1");
                for d in &o.drops {
                    ctx.set_global_alpha(0.35 + 0.5 * d.z);
                    ctx.begin_path();
                    let _ = ctx.arc(d.x + (t / 1000.0 * d.sway_w + d.sway).sin() * 14.0 * d.z, d.y, 0.7 + 2.2 * d.z, 0.0, 6.283);
                    ctx.fill();
                }
            } else {
                ctx.set_stroke_style_str("#d6e0e2");
                ctx.set_line_cap("round");
                for d in &o.drops {
                    ctx.set_global_alpha((0.12 + 0.3 * d.z) * self.fade);
                    ctx.set_line_width(if d.z > 0.8 { 1.3 } else { 1.0 });
                    ctx.begin_path();
                    ctx.move_to(d.x, d.y);
                    ctx.line_to(d.x - self.slant * d.len, d.y - d.len);
                    ctx.stroke();
                }
            }
        });
    }
}

fn start_precip() {
    stop_precip();
    let (w, _) = wx();
    let cv: HtmlCanvasElement = dom::by_id("precip").unchecked_into();
    let amount = w.rain.max(w.snow);
    cv.set_hidden(amount == 0.0);
    if amount == 0.0 {
        return;
    }
    let (sw, sh) = screen();
    let dpr = dom::win().device_pixel_ratio();
    let dpr = if dpr > 0.0 { dpr } else { 1.0 };
    cv.set_width((sw * dpr) as u32);
    cv.set_height((sh * dpr) as u32);
    let Some(ctx) = cv.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok()) else { return };
    let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    let snow = w.snow > w.rain;
    let p = Rc::new(Precip {
        ctx,
        w: sw,
        h: sh,
        snow,
        // sideways px per px fallen
        slant: if snow { 0.0 } else { 0.06 + w.wind * 0.4 },
        drift: if snow { 12.0 + w.wind * 70.0 } else { 0.0 },
        // against a dark sky every streak catches the eye, so night rain is fainter
        fade: if dom::has_class(&dom::root(), "night") { 0.65 } else { 1.0 },
    });
    let n = ((if snow { 300.0 } else { 520.0 }) * amount * (sw * sh) / (1440.0 * 900.0)).round() as usize;
    let drops: Vec<Drop> = (0..n).map(|_| p.spawn(true)).collect();
    ov(|o| o.drops = drops);
    if dom::reduce_motion() {
        p.draw(0.0);
        return;
    }
    let last = dom::perf_now();
    let h = dom::frame(move |now| precip_frame(p, last, now));
    ov(|o| o.precip_raf = Some(h));
}

fn precip_frame(p: Rc<Precip>, last: f64, now: f64) {
    let dt = ((now - last) / 1000.0).min(0.05);
    let mut drops = ov(|o| std::mem::take(&mut o.drops));
    for d in drops.iter_mut() {
        d.y += d.v * dt;
        d.x += (p.slant * d.v + p.drift * d.z) * dt;
        if d.y - d.len > p.h || d.x > p.w + 40.0 {
            *d = p.spawn(false);
        }
    }
    ov(|o| o.drops = drops);
    p.draw(now);
    let h = dom::frame(move |t| precip_frame(p, now, t));
    ov(|o| o.precip_raf = Some(h));
}

// ---------- life: birds by day, shooting stars by night ----------

fn soon(ms: f64, f: impl FnOnce() + 'static) {
    let h = dom::timeout(ms as i32, f);
    ov(|o| o.life_timers.push(h));
}

fn stop_life() {
    for t in ov(|o| std::mem::take(&mut o.life_timers)) {
        dom::clear_timeout(Some(t));
    }
    dom::clear(&dom::by_id("birds"));
    for n in dom::qa_in(&dom::by_id("scene"), ".flash") {
        n.remove();
    }
}

fn strike() {
    lightning();
    soon(6000.0 + dom::random() * 16000.0, strike);
}

fn meteor() {
    shooting_star();
    soon(9000.0 + dom::random() * 26000.0, meteor);
}

fn next_flock() {
    flock(false);
    soon(16000.0 + dom::random() * 22000.0, next_flock);
}

fn start_life() {
    stop_life();
    if dom::reduce_motion() {
        return;
    }
    let (w, _) = wx();
    let s = forest::sky(sky_hour(), &w);
    if w.storm {
        soon(1500.0 + dom::random() * 3500.0, strike);
    }
    // birds stay in the trees through rain, snow and thick fog
    let calm = w.rain < 0.15 && w.snow < 0.15 && !w.storm && w.fog < 0.5;
    if s.stars > 0.5 {
        soon(2500.0 + dom::random() * 7000.0, meteor);
    } else if calm && (s.day || forest::golden(sky_hour()) > 0.2) {
        // the first flock is already on its way when the pause appears, so short pauses still see it
        flock(true);
        soon(9000.0 + dom::random() * 12000.0, next_flock);
    }
}

fn flock(midway: bool) {
    let (w, hgt) = screen();
    let rnd = dom::random;
    let dir = if rnd() < 0.5 { 1.0 } else { -1.0 };
    let n = 2 + (rnd() * 8.0).floor() as u32;
    let near = rnd(); // 0 far and small, 1 closer and larger
    let size = 5.0 + near * 8.0;
    let speed = 22.0 + near * 30.0; // px per second
    let t = (w + 500.0) / speed;
    let f = h("div").class("flock").html();
    let horizon = dom::root().style().get_property_value("--horizon").unwrap_or_default();
    let horizon = if horizon.trim().is_empty() { "#dfe8de".to_string() } else { horizon.trim().to_string() };
    let vars = [
        ("--y", format!("{}px", fx(hgt * (0.07 + rnd() * 0.3), 0))),
        ("--t", format!("{}s", fx(t, 1))),
        ("--d", if midway { format!("{}s", fx(-t * (0.15 + rnd() * 0.35), 1)) } else { "0s".into() }),
        ("--x0", if dir > 0.0 { "-250px".into() } else { format!("{}px", forest::num(w + 250.0)) }),
        ("--x1", if dir > 0.0 { format!("{}px", forest::num(w + 250.0)) } else { "-250px".into() }),
        ("--bt", format!("{}s", fx(5.0 + rnd() * 5.0, 1))),
        ("--bob", format!("{}px", fx(8.0 + rnd() * 22.0, 0))),
        ("--bird", forest::mix(&horizon, "#0a1c15", 0.62 + near * 0.25)),
    ];
    for (k, v) in vars {
        dom::set_var(&f, k, &v);
    }
    let mut svg = s("svg").attr("width", "1").attr("height", "1");
    // a loose V behind the leader, or a scatter
    let vee = n > 3 && rnd() < 0.6;
    for k in 0..n {
        let row = k.div_ceil(2) as f64;
        let side = if k % 2 == 1 { 1.0 } else { -1.0 };
        let x = if vee { -dir * row * size * 2.6 + (rnd() - 0.5) * size } else { (rnd() - 0.5) * size * 12.0 };
        let y = if vee { side * row * size * 1.5 + (rnd() - 0.5) * size * 0.6 } else { (rnd() - 0.5) * size * 6.0 };
        let sz = size * (0.8 + rnd() * 0.4);
        svg = svg.child(bird(x, y, sz));
    }
    f.append_child(&svg.el()).unwrap();
    let me = f.clone();
    dom::on::<AnimationEvent>(&f, "animationend", move |e| {
        if e.target().is_some_and(|t| wasm_bindgen::JsValue::from(t) == wasm_bindgen::JsValue::from(me.clone())) {
            me.remove();
        }
    });
    dom::by_id("birds").append_child(&f).unwrap();
}

/// One bird: a curved gull silhouette whose wings beat a few times, then glide.
fn bird(x: f64, y: f64, s0: f64) -> B {
    let wing = |tip: f64, bend: f64| {
        format!(
            "M{} {}Q{} {} 0 0Q{} {} {} {}",
            forest::num(-s0),
            fx(tip * s0, 2),
            fx(-s0 * 0.45, 2),
            fx(bend * s0, 2),
            fx(s0 * 0.45, 2),
            fx(bend * s0, 2),
            forest::num(s0),
            fx(tip * s0, 2)
        )
    };
    let (up, mid, down) = (wing(-0.6, -0.55), wing(-0.08, -0.38), wing(0.42, -0.12));
    let dur = 2.2 + dom::random() * 1.6;
    let values = [&mid, &up, &down, &up, &down, &mid, &mid].map(|s| s.as_str()).join(";");
    s("path")
        .attr("d", &mid)
        .attr("transform", &format!("translate({} {})", fx(x, 1), fx(y, 1)))
        .attr("stroke-width", &fx((s0 * 0.16).max(1.1), 2))
        .child(
            s("animate")
                .attr("attributeName", "d")
                .attr("dur", &format!("{}s", fx(dur, 2)))
                .attr("repeatCount", "indefinite")
                .attr("begin", &format!("{}s", fx(-dom::random() * dur, 2)))
                .attr("values", &values)
                .attr("keyTimes", "0;0.09;0.2;0.31;0.42;0.55;1")
                .attr("calcMode", "spline")
                .attr("keySplines", &["0.45 0 0.55 1"; 6].join(";")),
        )
}

fn lightning() {
    let f = h("div").class("flash").html();
    dom::set_var(&f, "--fx", &format!("{}%", fx(15.0 + dom::random() * 70.0, 0)));
    dom::set_var(&f, "--peak", &fx(0.22 + dom::random() * 0.14, 2));
    let me = f.clone();
    dom::on::<Event>(&f, "animationend", move |_| me.remove());
    let _ = dom::by_id("scene").insert_before(&f, Some(&dom::by_id("birds")));
}

fn shooting_star() {
    let (w, hgt) = screen();
    let rnd = dom::random;
    let m = h("div").class("meteor").html();
    let left = rnd() < 0.5;
    let x = w * if left { 0.08 + rnd() * 0.35 } else { 0.55 + rnd() * 0.3 };
    dom::set_style(&m, "left", &format!("{}px", fx(x, 0)));
    dom::set_style(&m, "top", &format!("{}px", fx(hgt * (0.04 + rnd() * 0.22), 0)));
    let a = if left { 18.0 + rnd() * 20.0 } else { 142.0 + rnd() * 20.0 };
    dom::set_var(&m, "--a", &format!("{}deg", forest::num(a)));
    dom::set_var(&m, "--len", &format!("{}px", fx(160.0 + rnd() * 220.0, 0)));
    let me = m.clone();
    dom::on::<Event>(&m, "animationend", move |_| me.remove());
    let _ = dom::by_id("scene").insert_before(&m, Some(&dom::by_id("clouds")));
}

// ---------- bedtime reminder ----------
// The same forest, with a still sun or moon showing the time left before the sleep time. The
// window is opened by the app at 2 h, 1 h, 30, 15, 5 and 0 minutes; from then until two hours
// after it, each reminder asks when to come back; the first choice is what Enter and Esc pick.

fn plural(n: i64, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

const BEDTIME_NOTES: [(i64, &str); 6] = [
    (0, "Good night."),
    (5, "Almost time. Lights low, screens down."),
    (15, "Time to get ready for bed."),
    (30, "Maybe put the screens away soon."),
    (60, "Maybe wrap up what you're in the middle of."),
    (i64::MAX, "A good time to start winding down."),
];

const LATE_NOTES: [(i64, &str); 3] = [
    (30, "Time to put the screen down."),
    (60, "Whatever it is, it can wait until morning."),
    (i64::MAX, "Close the laptop. Tomorrow will thank you."),
];

fn note(notes: &[(i64, &'static str)], m: i64) -> &'static str {
    notes.iter().find(|(max, _)| m <= *max).map(|n| n.1).unwrap_or("")
}

fn snooze_label(m: u32) -> String {
    if m == 60 {
        "1 hour".into()
    } else {
        format!("{m} min")
    }
}

/// Minutes since the sleep time, once its own five minutes are over; None before that.
fn minutes_late(p: &Bedtime) -> Option<i64> {
    let late = ((dom::now_ms() - p.bedtime_ts as f64 * 1000.0) / 60000.0).floor() as i64;
    (p.offset < 0 || late >= 5).then(|| late.max(1))
}

fn span(m: i64) -> String {
    if m < 60 {
        plural(m, "minute")
    } else if m % 60 != 0 {
        format!("{} {}", plural(m / 60, "hour"), plural(m % 60, "minute"))
    } else {
        plural(m / 60, "hour")
    }
}

/// The minutes the screen speaks of: the reminder's own mark while it is fresh, the real time left
/// once it has been sitting there longer than its five minutes.
fn bedtime_minutes(p: &Bedtime) -> i64 {
    let left = ((p.bedtime_ts as f64 * 1000.0 - dom::now_ms()) / 60000.0).ceil().max(0.0) as i64;
    let offset = p.offset as i64;
    if offset > 0 && left > offset - 5 {
        offset
    } else {
        left
    }
}

fn text(id: &str, t: &str) {
    dom::set_text(&dom::by_id(id), t);
}

fn render_bedtime() {
    let Some(p) = ov(|o| match &o.current {
        Some(Cur::Bed(b)) => Some(b.clone()),
        _ => None,
    }) else {
        return;
    };
    if let Some(late) = minutes_late(&p) {
        text("count", &if late < 60 { late.to_string() } else { format!("{}:{:02}", late / 60, late % 60) });
        text("cue", if late < 60 { if late == 1 { "minute late" } else { "minutes late" } } else { "hours late" });
        let sp = span(late);
        let mut c = sp.chars();
        let cap = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
        text("title", &format!("{cap} past bedtime."));
        text("detail", &format!("You planned to sleep at {}. {}", p.bedtime, note(&LATE_NOTES, late)));
        return;
    }
    let m = bedtime_minutes(&p);
    let hours = m >= 120 && m % 60 == 0;
    text("count", &if m == 0 { "now".to_string() } else { (if hours { m / 60 } else { m }).to_string() });
    text("cue", if m == 0 { "" } else if hours { "hours" } else if m == 1 { "minute" } else { "minutes" });
    let title = if m == 0 {
        "It's bedtime.".to_string()
    } else if m == 60 {
        "Bedtime in 1 hour.".to_string()
    } else {
        format!("Bedtime in {}.", if hours { plural(m / 60, "hour") } else { plural(m, "minute") })
    };
    text("title", &title);
    text("detail", &format!("You planned to sleep at {}. {}", p.bedtime, note(&BEDTIME_NOTES, m)));
}

fn show_bedtime(p: Bedtime) {
    // each reminder walks into a different stretch of forest; an update keeps the one in view
    if !dom::has_class(&dom::body(), "in") {
        ov(|o| o.scene_seed = new_seed());
    }
    ov(|o| {
        o.weather_now = p.weather.clone();
        o.current = Some(Cur::Bed(p.clone()));
    });
    paint();
    start_life();
    dom::by_id("breath").set_class_name("breath settled");
    let _ = dom::body().class_list().remove_2("waiting", "breathing");
    render_bedtime();
    dom::clear_interval(ov(|o| o.bedtime_timer.take()));
    let t = dom::interval(30000, render_bedtime);
    ov(|o| o.bedtime_timer = Some(t));
    draw_snooze(&p);
    let _ = dom::by_id("back").focus();
    dom::frame(|_| {
        let _ = dom::body().class_list().add_1("in");
    });
}

/// At and after the sleep time the main button is the default wait, with the others beside it.
fn draw_snooze(p: &Bedtime) {
    let later = dom::by_id("later");
    for b in dom::qa_in(&later, "button") {
        b.remove();
    }
    let opts = &p.snooze;
    later.set_hidden(opts.len() < 2);
    let first = opts.first().copied();
    dom::by_id("back").set_text_content(Some(&match first {
        Some(m) => format!("Remind me in {}", snooze_label(m)),
        None => "Got it".into(),
    }));
    if let Some(hint) = dom::q(".hint") {
        hint.set_text_content(Some(&match first {
            Some(m) => format!("Press Enter or Esc for {}", snooze_label(m)),
            None => "Press Esc to dismiss".into(),
        }));
    }
    for &m in opts.iter().skip(1) {
        let b = h("button").text(&snooze_label(m)).on::<Event>("click", move |_| invoke_or_error("bedtime_later", json!({ "minutes": m })));
        later.append_child(&b.el()).unwrap();
    }
}

fn hide_bedtime() {
    dom::clear_interval(ov(|o| o.bedtime_timer.take()));
    stop_life();
    stop_precip();
    ov(|o| o.current = None);
    let _ = dom::body().class_list().remove_1("in");
}

/// The main button, Enter and Esc: the default wait when there is a choice, else just dismiss.
fn bedtime_ok() {
    let first = ov(|o| match &o.current {
        Some(Cur::Bed(b)) => b.snooze.first().copied(),
        _ => None,
    });
    match first {
        Some(m) => invoke_or_error("bedtime_later", json!({ "minutes": m })),
        None => invoke_or_error("bedtime_ok", tauri::none()),
    }
}

fn close() {
    if ov(|o| o.bedtime_mode) {
        bedtime_ok()
    } else {
        back()
    }
}

// ---------- boot ----------

fn bind() {
    dom::on::<Event>(&dom::by_id("back"), "click", |_| close());
    // the point is to write it out, so no pasting or dropping text in
    let t = typed();
    for kind in ["paste", "drop", "dragover"] {
        dom::on::<Event>(&t, kind, |e| e.prevent_default());
    }
    dom::on::<InputEvent>(&t, "beforeinput", |e| {
        if matches!(e.input_type().as_str(), "insertFromPaste" | "insertFromDrop" | "insertFromYank") {
            e.prevent_default();
        }
    });
    {
        let t2 = t.clone();
        dom::on::<Event>(&t, "input", move |_| {
            autosize(&t2);
            sync_unlock();
            set_error("");
            let u = ov(|o| o.unlock.clone());
            if u.typing {
                let value = t2.value();
                ov(|o| o.draft = u.draft_key.map(|k| (k, value)));
            }
        });
    }
    // Return opens it for the shortest time once the answer is accepted; before that it points at what's missing.
    {
        let t2 = t.clone();
        dom::on::<KeyboardEvent>(&t, "keydown", move |e| {
            if e.key() != "Enter" || e.is_composing() {
                return;
            }
            e.prevent_default();
            let u = ov(|o| {
                let pausing = matches!(o.current, Some(Cur::Pause(_)));
                pausing.then(|| o.unlock.clone())
            });
            if let Some(u) = u {
                if let Some(m) = u.shortest.filter(|&m| m > 0) {
                    if unlocks(u.prompt.as_ref(), &t2.value()) {
                        cont(m);
                        return;
                    }
                }
            }
            dom::restart_class(&dom::by_id("unlock-hint"), "nudge");
        });
    }
    dom::on::<AnimationEvent>(&dom::by_id("unlock-hint"), "animationend", |e| {
        if let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            let _ = el.class_list().remove_1("nudge");
        }
    });
    // Esc or Cmd+W closes the tab. The app menu has no Close Window item, so the page sees Cmd+W.
    dom::on::<KeyboardEvent>(&dom::doc(), "keydown", |e| {
        let bedtime = ov(|o| o.bedtime_mode);
        let key = e.key();
        if key == "Escape" {
            close();
        } else if bedtime && key == "Enter" && dom::doc().active_element().is_some_and(|a| a.is_same_node(Some(&dom::body()))) {
            close();
        } else if e.meta_key() && !e.ctrl_key() && !e.alt_key() && !e.shift_key() && key.to_lowercase() == "w" {
            e.prevent_default();
            close();
        }
    });
    dom::on::<Event>(&dom::win(), "resize", |_| {
        build_scene();
        build_weather();
    });
    dom::interval(60000, || {
        if ov(|o| o.current.is_none()) {
            return;
        }
        tauri::spawn(async {
            if dom::param("weather").is_none() {
                let r: Option<Reading> = tauri::invoke("weather_now", tauri::none()).await.ok().flatten();
                ov(|o| o.weather_now = r);
            }
            if ov(|o| o.current.is_some()) && paint() {
                start_life();
            }
        });
    });
}

fn demo_pause(reason: &str) {
    let site = "youtube.com";
    let prompt = match dom::param("prompt").as_deref().unwrap_or("reflect") {
        "cost" => Some(Prompt { kind: "cost".into(), text: format!("This is visit number 3 to {site} today. What did the last one give you?"), sentence: None }),
        "commit" => Some(Prompt {
            kind: "commit".into(),
            text: "To unlock, type this sentence exactly:".into(),
            sentence: Some(dom::param("sentence").filter(|s| !s.is_empty()).unwrap_or_else(|| format!("I could close this tab, but I'm choosing {site} instead."))),
        }),
        "reflect" => Some(Prompt { kind: "reflect".into(), text: format!("What were you about to do before you opened {site}?"), sentence: None }),
        _ => None,
    };
    let seconds = dom::param("seconds").and_then(|s| s.parse::<f64>().ok()).filter(|&s| s > 0.0).unwrap_or(6.0);
    show(Intercept {
        rule_id: "demo".into(),
        rule_name: "Social & video".into(),
        site: site.into(),
        reason: reason.into(),
        pause_seconds: seconds as u32,
        countdown: dom::param("countdown").filter(|c| !c.is_empty()).unwrap_or_else(|| "breathe".into()),
        prompt,
        waited: dom::param("waited").as_deref() == Some("1"),
        opens_today: 2,
        minutes_today: 12,
        daily_opens: Some(4),
        daily_minutes: Some(30),
        session_options: vec![2, 5, 15],
        until: (reason == "blocked").then(|| "17:00".into()),
        ..Intercept::default()
    });
}

fn demo_bedtime() {
    // negative minutes: that long past bedtime
    let minutes = dom::param("minutes").map(|m| m.parse::<f64>().unwrap_or(0.0)).unwrap_or(30.0);
    let when = js_sys::Date::new(&(dom::now_ms() + minutes * 60000.0).into());
    let clock = dom::param("at").filter(|a| !a.is_empty()).unwrap_or_else(|| format!("{:02}:{:02}", when.get_hours(), when.get_minutes()));
    show_bedtime(Bedtime {
        offset: minutes as i32,
        minutes_left: minutes.max(0.0) as i64,
        bedtime: clock,
        bedtime_ts: (when.get_time() / 1000.0).floor() as i64,
        snooze: if minutes <= 0.0 { vec![15, 30, 45, 60] } else { vec![] },
        sky: "time".into(),
        weather: None,
    });
}

pub fn start(bedtime: bool) {
    dom::add_style(include_str!("../styles/overlay.css"));
    ov(|o| {
        o.bedtime_mode = bedtime;
        o.scene_seed = new_seed();
    });
    build();
    bind();

    if bedtime {
        let _ = dom::body().class_list().add_1("bedtime");
        let later = dom::by_id("later");
        later.set_hidden(true);
        dom::q_in(&later, "span").set_text_content(Some("or in"));
        dom::by_id("commit").set_hidden(true);
        dom::by_id("back").set_text_content(Some("Got it"));
        dom::by_id("mute-row").set_hidden(false);
        dom::on::<Event>(&dom::by_id("mute"), "click", |_| invoke_or_error("bedtime_mute", tauri::none()));
        if let Some(hint) = dom::q(".hint") {
            hint.set_text_content(Some("Press Esc to dismiss"));
        }
        tauri::listen("bedtime", show_bedtime);
        tauri::listen::<serde_json::Value>("bedtime_hide", |_| hide_bedtime());
        tauri::spawn(async {
            if let Ok(Some(p)) = tauri::invoke::<Option<Bedtime>>("bedtime_state", tauri::none()).await {
                show_bedtime(p);
            }
        });
    } else {
        tauri::listen("intercept", show);
        tauri::listen::<serde_json::Value>("dismiss", |_| {
            stop_countdown();
            stop_life();
            stop_precip();
            ov(|o| o.current = None);
            let _ = dom::body().class_list().remove_3("in", "waiting", "breathing");
        });
        tauri::spawn(async {
            if let Ok(Some(i)) = tauri::invoke::<Option<Intercept>>("overlay_state", tauri::none()).await {
                show(i);
            }
        });
    }

    // test hook: ?w=overlay&demo=pause|expired|blocked|open_limit|time_limit
    //   &countdown=breathe|fill|drain   &prompt=reflect|cost|commit|none   &seconds=6   &sentence=…
    //   &waited=1 (arrive with the countdown already served)   &hour=21.5 (light of that hour)
    //   &weather=clear|cloudy|overcast|fog|drizzle|rain|downpour|storm|snow   &wind=30 (km/h)
    //   bedtime: ?w=overlay&mode=bedtime&demo=bedtime&minutes=30&hour=21&weather=clear (&at=23:00)
    match dom::param("demo").filter(|d| !d.is_empty()) {
        Some(d) if bedtime && d == "bedtime" => demo_bedtime(),
        Some(d) if !bedtime => demo_pause(&d),
        _ => {}
    }
}
