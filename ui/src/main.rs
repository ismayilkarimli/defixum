//! Defixum's windows, in Rust compiled to WebAssembly. One page, routed by `?w=`: the settings
//! window (no `w`), the pause, bedtime and get-up screens (`overlay`, by `mode`) and the toast.

mod country;
mod dashboard;
mod dom;
mod overlay;
mod sky;
mod tauri;
mod toast;
mod types;

fn main() {
    match dom::param("w").as_deref() {
        Some("overlay") => {
            let mode = dom::param("mode").unwrap_or_default();
            dom::doc().set_title(match mode.as_str() {
                "bedtime" => "Defixum bedtime",
                "move" => "Defixum break",
                _ => "Defixum pause",
            });
            overlay::start(&mode);
        }
        Some("toast") => toast::start(),
        _ => dashboard::start(),
    }
}
