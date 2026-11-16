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
}

fn tick() {
    let s = ((UNTIL.get() - dom::now_ms()) / 1000.0).ceil().max(0.0) as u64;
    dom::set_text(&dom::by_id("left"), &format!("{}:{:02}", s / 60, s % 60));
}

fn show(t: Toast) {
    dom::clear_interval(TIMER.take());
    let body = dom::body();
    let moving = t.kind == "move";
    KIND.with(|k| *k.borrow_mut() = t.kind.clone());
    dom::toggle_class(&body, "move", moving);
    let head = dom::by_id("head");
    if moving {
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
    hold_bar(!moving && t.holding, t.hold_ms);
    // restart the fade, so a refused press while it is up gives it the full time again
    dom::restart_class(&body, "on");
}

/// The bar that fills while ⌘Tab is held down toward getting through; empty when let go.
fn hold_bar(holding: bool, ms: u32) {
    let body = dom::body();
    let fill = dom::by_id("fill");
    dom::toggle_class(&body, "holding", holding);
    // back to empty at once, then fill over the hold
    dom::set_style(&fill, "transition", "none");
    dom::set_style(&fill, "transform", "scaleX(0)");
    let _ = fill.offset_width();
    if holding {
        let ms = if ms == 0 { 5000 } else { ms };
        dom::set_style(&fill, "transition", &format!("transform {ms}ms linear"));
        dom::set_style(&fill, "transform", "scaleX(1)");
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
    let bar = h("span").class("bar").attr("aria-hidden", "true").child(h("span").id("fill"));
    body.append_child(&bar.el()).unwrap();
    // only the note to get up takes clicks: one click puts it away
    dom::on::<web_sys::Event>(&body, "click", |_| {
        if KIND.with(|k| k.borrow().as_str() == "move") {
            tauri::spawn(async {
                let _ = tauri::call("move_close", tauri::none()).await;
            });
        }
    });
    match dom::param("demo").filter(|d| !d.is_empty()).as_deref() {
        Some("move") => show(Toast { kind: "move".into(), minutes: 45, seconds: 10, ..Toast::default() }),
        Some(d) => show(Toast {
            kind: "switch".into(),
            until: dom::now_ms() + 58000.0,
            limit: 6,
            holding: d == "hold",
            hold_ms: 5000,
            ..Toast::default()
        }),
        None => tauri::listen("toast", show),
    }
}
