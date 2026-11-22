//! The toast on a dark native blur: how long ⌘Tab rests, or a quiet note to get up when a video
//! or call is on and the full screen would be in the way.

use std::cell::{Cell, RefCell};

use crate::dom::{self, h};
use crate::tauri;
use crate::types::Toast;

thread_local! {
    static UNTIL: Cell<f64> = const { Cell::new(0.0) };
    static TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static KIND: RefCell<String> = const { RefCell::new(String::new()) };
    static HOLD_RAF: Cell<Option<i32>> = const { Cell::new(None) };
}

fn tick() {
    let s = ((UNTIL.get() - dom::now_ms()) / 1000.0).ceil().max(0.0) as u64;
    dom::set_text(&dom::by_id("left"), &format!("{}:{:02}", s / 60, s % 60));
}

fn show(t: Toast) {
    dom::clear_interval(TIMER.take());
    let body = dom::body();
    let moving = t.kind == "move";
    let extend = t.kind == "extend";
    KIND.with(|k| *k.borrow_mut() = t.kind.clone());
    dom::toggle_class(&body, "move", moving);
    dom::toggle_class(&body, "extend", extend);
    let head = dom::by_id("head");
    if extend {
        UNTIL.set(t.until);
        dom::clear(&head);
        head.append_with_str_1("Time's up in ").unwrap();
        head.append_child(&h("span").id("left").el()).unwrap();
        dom::set_var(&body, "--life", &format!("{}s", t.seconds.max(3) + 1));
        tick();
        TIMER.set(Some(dom::interval(250, tick)));
    } else if moving {
        dom::set_text(&head, &format!("{} minutes at the screen", t.minutes));
        dom::set_text(&dom::by_id("why"), "Stand up and stretch when you get a chance.");
        dom::set_var(&body, "--life", &format!("{}s", t.seconds.max(3)));
    } else {
        UNTIL.set(t.until);
        dom::clear(&head);
        head.append_with_str_1("⌘Tab is resting for ").unwrap();
        head.append_child(&h("span").id("left").el()).unwrap();
        dom::set_text(&dom::by_id("why"), "Hold ⌘Tab for 5 seconds to switch anyway.");
        dom::set_var(&body, "--life", "5s");
        tick();
        TIMER.set(Some(dom::interval(500, tick)));
    }
    let was_holding = dom::has_class(&body, "holding");
    let holding = t.kind == "switch" && t.hold_started > 0.0;
    hold_bar(if holding { t.hold_started } else { 0.0 }, t.hold_ms);
    // let go while it is up: no second fade-in, it just lingers and goes
    dom::toggle_class(&body, "linger", was_holding && !holding);
    // restart the fade, so a refused press while it is up gives it the full time again
    dom::restart_class(&body, "on");
}

/// The bar that fills while ⌘Tab is held down toward getting through. It is drawn from when the
/// hold began, every frame, so it is full right as the hold gets through; let go and it drains.
fn hold_bar(started: f64, ms: u32) {
    dom::cancel_frame(HOLD_RAF.take());
    let fill = dom::by_id("fill");
    dom::toggle_class(&dom::body(), "holding", started > 0.0);
    if started > 0.0 {
        dom::set_style(&fill, "transition", "none");
        let total = if ms == 0 { 5000.0 } else { ms as f64 };
        fill_frame(started, total);
    } else {
        // from wherever it got to, quickly back to empty
        dom::set_style(&fill, "transition", "transform 180ms cubic-bezier(0.3, 0, 0.2, 1)");
        dom::set_style(&fill, "transform", "scaleX(0)");
    }
}

fn fill_frame(started: f64, total: f64) {
    let p = ((dom::now_ms() - started) / total).clamp(0.0, 1.0);
    dom::set_style(&dom::by_id("fill"), "transform", &format!("scaleX({p:.4})"));
    if p < 1.0 {
        HOLD_RAF.set(Some(dom::frame(move |_| fill_frame(started, total))));
    }
}

pub fn start() {
    dom::add_style(include_str!("../styles/toast.css"));
    let status = h("span")
        .attr("role", "status")
        .child(h("strong").id("head").text("⌘Tab is resting for ").child(h("span").id("left").text("1:00")))
        .child(h("small").id("why").text("Stay with what you have open."));
    let body = dom::body();
    body.append_child(&status.el()).unwrap();
    body.append_child(&h("button").id("more").attr("type", "button").text("+1 minute").el()).unwrap();
    let bar = h("span").class("bar").attr("aria-hidden", "true").child(h("span").id("fill"));
    body.append_child(&bar.el()).unwrap();
    // only the note to get up and the session's last seconds take clicks: one click puts the
    // first away, and gives the second one more minute
    dom::on::<web_sys::Event>(&body, "click", |_| {
        let kind = KIND.with(|k| k.borrow().clone());
        let cmd = match kind.as_str() {
            "move" => "move_close",
            _ => return,
        };
        tauri::spawn(async move {
            let _ = tauri::call(cmd, tauri::none()).await;
        });
    });
    dom::on::<web_sys::Event>(&dom::by_id("more"), "click", |_| {
        tauri::spawn(async {
            let _ = tauri::call("session_extend", tauri::none()).await;
        });
    });
    match dom::param("demo").filter(|d| !d.is_empty()).as_deref() {
        Some("move") => show(Toast { kind: "move".into(), minutes: 45, seconds: 10, ..Toast::default() }),
        Some("extend") => show(Toast { kind: "extend".into(), until: dom::now_ms() + 5000.0, seconds: 5, ..Toast::default() }),
        Some(d) => show(Toast {
            kind: "switch".into(),
            until: dom::now_ms() + 58000.0,
            limit: 6,
            hold_started: if d == "hold" { dom::now_ms() } else { 0.0 },
            hold_ms: 5000,
            ..Toast::default()
        }),
        None => tauri::listen("toast", show),
    }
}
