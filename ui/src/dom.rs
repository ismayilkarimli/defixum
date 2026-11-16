//! Thin helpers over web-sys: finding and building elements, listeners and timers.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, Window};

const SVG_NS: &str = "http://www.w3.org/2000/svg";

pub fn win() -> Window {
    web_sys::window().unwrap()
}

pub fn doc() -> Document {
    win().document().unwrap()
}

pub fn root() -> HtmlElement {
    doc().document_element().unwrap().unchecked_into()
}

pub fn body() -> HtmlElement {
    doc().body().unwrap()
}

/// The element with this id. Panics if it is missing: every id is created by this crate.
pub fn by_id(id: &str) -> HtmlElement {
    doc().get_element_by_id(id).unwrap_or_else(|| panic!("no #{id}")).unchecked_into()
}

pub fn input(id: &str) -> HtmlInputElement {
    by_id(id).unchecked_into()
}

pub fn q(sel: &str) -> Option<HtmlElement> {
    doc().query_selector(sel).ok().flatten().map(|e| e.unchecked_into())
}

pub fn qa(sel: &str) -> Vec<HtmlElement> {
    qa_in(&doc().document_element().unwrap(), sel)
}

pub fn qa_in(parent: &Element, sel: &str) -> Vec<HtmlElement> {
    let list = parent.query_selector_all(sel).unwrap();
    (0..list.length()).filter_map(|i| list.item(i)).map(|n| n.unchecked_into()).collect()
}

pub fn q_in(parent: &Element, sel: &str) -> HtmlElement {
    parent.query_selector(sel).ok().flatten().unwrap_or_else(|| panic!("no {sel}")).unchecked_into()
}

/// A query parameter of the page's address.
pub fn param(name: &str) -> Option<String> {
    let search = win().location().search().unwrap_or_default();
    web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(name)
}

pub fn reduce_motion() -> bool {
    win()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn perf_now() -> f64 {
    win().performance().map(|p| p.now()).unwrap_or_else(now_ms)
}

pub fn random() -> f64 {
    js_sys::Math::random()
}

/// Adds a listener that lives as long as the page.
pub fn on<E: JsCast + 'static>(target: &web_sys::EventTarget, event: &str, f: impl FnMut(E) + 'static) {
    let mut f = f;
    let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| f(e.unchecked_into()));
    target.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref()).unwrap();
    cb.forget();
}

pub fn timeout(ms: i32, f: impl FnOnce() + 'static) -> i32 {
    let cb = Closure::once_into_js(f);
    win().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms).unwrap()
}

pub fn clear_timeout(handle: Option<i32>) {
    if let Some(h) = handle {
        win().clear_timeout_with_handle(h);
    }
}

pub fn interval(ms: i32, f: impl FnMut() + 'static) -> i32 {
    let cb = Closure::<dyn FnMut()>::new(f);
    let h = win().set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), ms).unwrap();
    cb.forget();
    h
}

pub fn clear_interval(handle: Option<i32>) {
    if let Some(h) = handle {
        win().clear_interval_with_handle(h);
    }
}

pub fn frame(f: impl FnOnce(f64) + 'static) -> i32 {
    let cb = Closure::once_into_js(f);
    win().request_animation_frame(cb.unchecked_ref()).unwrap()
}

pub fn cancel_frame(handle: Option<i32>) {
    if let Some(h) = handle {
        let _ = win().cancel_animation_frame(h);
    }
}

pub fn set_var(el: &HtmlElement, name: &str, value: &str) {
    let _ = el.style().set_property(name, value);
}

pub fn set_style(el: &HtmlElement, name: &str, value: &str) {
    let _ = el.style().set_property(name, value);
}

pub fn toggle_class(el: &Element, class: &str, on: bool) {
    let _ = el.class_list().toggle_with_force(class, on);
}

pub fn has_class(el: &Element, class: &str) -> bool {
    el.class_list().contains(class)
}

pub fn set_text(el: &Element, text: &str) {
    if el.text_content().as_deref() != Some(text) {
        el.set_text_content(Some(text));
    }
}

pub fn clear(el: &Element) {
    el.set_text_content(None);
}

/// Restarts a CSS animation tied to `class` by removing it and forcing a layout first.
pub fn restart_class(el: &HtmlElement, class: &str) {
    let _ = el.class_list().remove_1(class);
    let _ = el.offset_width();
    let _ = el.class_list().add_1(class);
}

pub fn is_active(el: &Element) -> bool {
    doc().active_element().is_some_and(|a| &a == el)
}

// ---------- building ----------

/// An element under construction: `h("div").class("x").text("hi").into()`.
pub struct B(pub Element);

/// An HTML element.
pub fn h(tag: &str) -> B {
    B(doc().create_element(tag).unwrap())
}

/// An SVG element.
pub fn s(tag: &str) -> B {
    B(doc().create_element_ns(Some(SVG_NS), tag).unwrap())
}

impl B {
    pub fn class(self, c: &str) -> Self {
        self.0.set_attribute("class", c).unwrap();
        self
    }
    pub fn id(self, id: &str) -> Self {
        self.0.set_id(id);
        self
    }
    pub fn attr(self, k: &str, v: &str) -> Self {
        self.0.set_attribute(k, v).unwrap();
        self
    }
    /// A boolean attribute, present when `on`.
    pub fn flag(self, k: &str, on: bool) -> Self {
        if on {
            self.0.set_attribute(k, "").unwrap();
        }
        self
    }
    pub fn text(self, t: &str) -> Self {
        self.0.append_with_str_1(t).unwrap();
        self
    }
    pub fn child(self, c: impl Into<Element>) -> Self {
        self.0.append_child(&c.into()).unwrap();
        self
    }
    pub fn kids(self, cs: impl IntoIterator<Item = B>) -> Self {
        for c in cs {
            self.0.append_child(&c.0).unwrap();
        }
        self
    }
    pub fn var(self, k: &str, v: &str) -> Self {
        if let Some(el) = self.0.dyn_ref::<HtmlElement>() {
            set_var(el, k, v);
        } else if let Some(el) = self.0.dyn_ref::<web_sys::SvgElement>() {
            let _ = el.style().set_property(k, v);
        }
        self
    }
    pub fn on<E: JsCast + 'static>(self, event: &str, f: impl FnMut(E) + 'static) -> Self {
        on(&self.0, event, f);
        self
    }
    pub fn el(self) -> Element {
        self.0
    }
    pub fn html(self) -> HtmlElement {
        self.0.unchecked_into()
    }
}

impl From<B> for Element {
    fn from(b: B) -> Element {
        b.0
    }
}

/// Puts a stylesheet compiled into the binary on the page.
pub fn add_style(css: &str) {
    let style = doc().create_element("style").unwrap();
    style.set_text_content(Some(css));
    doc().head().unwrap().append_child(&style).unwrap();
}
