//! The ⌘Tab toast: how long app switching rests, on a dark native blur.

use std::cell::Cell;

use crate::dom::{self, h};
use crate::tauri;
use crate::types::Toast;

thread_local! {
    static UNTIL: Cell<f64> = const { Cell::new(0.0) };
    static TIMER: Cell<Option<i32>> = const { Cell::new(None) };
}

fn tick() {
    let s = ((UNTIL.get() - dom::now_ms()) / 1000.0).ceil().max(0.0) as u64;
    dom::set_text(&dom::by_id("left"), &format!("{}:{:02}", s / 60, s % 60));
}

fn show(t: Toast) {
    UNTIL.set(t.until);
    let plural = if t.limit == 1 { "" } else { "es" };
    dom::set_text(&dom::by_id("why"), &format!("More than {} switch{plural} in a minute. Stay here a while.", t.limit));
    tick();
    dom::clear_interval(TIMER.take());
    TIMER.set(Some(dom::interval(500, tick)));
    // restart the fade, so a refused press while it is up gives it the full five seconds again
    dom::restart_class(&dom::body(), "on");
}

pub fn start() {
    dom::add_style(include_str!("../styles/toast.css"));
    let status = h("span")
        .attr("role", "status")
        .child(h("strong").text("⌘Tab is resting for ").child(h("span").id("left").text("1:00")))
        .child(h("small").id("why").text("Stay with what you have open."));
    dom::body().append_child(&status.el()).unwrap();
    if dom::param("demo").is_some_and(|d| !d.is_empty()) {
        show(Toast { until: dom::now_ms() + 58000.0, limit: 6 });
    } else {
        tauri::listen("toast", show);
    }
}
