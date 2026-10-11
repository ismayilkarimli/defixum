#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--probe") {
        let secs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(10);
        defixum_lib::probe(secs, args.get(i + 2).cloned());
        return;
    }
    defixum_lib::run()
}
