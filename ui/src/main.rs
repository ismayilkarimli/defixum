//! Defixum's windows, in Rust compiled to WebAssembly. One page, routed by `?w=`: the settings
//! window (no `w`), the pause and bedtime screens (`overlay`) and the ⌘Tab toast (`toast`).

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
            let bedtime = dom::param("mode").as_deref() == Some("bedtime");
            dom::doc().set_title(if bedtime { "Defixum bedtime" } else { "Defixum pause" });
            overlay::start(bedtime);
        }
        Some("toast") => toast::start(),
        _ => dashboard::start(),
    }
}
