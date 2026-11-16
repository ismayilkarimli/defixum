//! The page a tab is sent to when Defixum can't close it: the pause screen's sky for this hour
//! and a spruce line, drawn here so the page itself needs no script.

use chrono::{DateTime, Datelike, Local, Timelike};

/// The tree line is drawn wide and kept to the bottom of the window; narrower windows crop its sides.
const W: f64 = 3200.0;
const H: f64 = 300.0;

const STYLE: &str = r#"
* { margin: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; }
body {
  background: linear-gradient(to bottom, var(--sky-top), var(--horizon) 68%);
  color: var(--ink);
  font: 15px/1.5 -apple-system, system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  display: grid;
  place-items: center;
  text-align: center;
  padding: 2rem 1rem 22vh;
  animation: in 700ms ease both;
}
main { position: relative; z-index: 1; }
h1 {
  font-family: Optima, "Optima Nova", Candara, "Segoe UI", sans-serif;
  font-weight: 400;
  font-size: clamp(28px, 5vw, 36px);
  line-height: 1.18;
  text-wrap: balance;
  text-shadow: var(--lift);
}
.site { color: var(--site); }
p { color: var(--muted); margin: 1rem auto 0; max-width: 30rem; text-shadow: var(--lift); }
.trees { position: fixed; left: 0; bottom: 0; width: 100%; height: 30vh; display: block; }
@keyframes in { from { opacity: 0; } to { opacity: 1; } }
@media (prefers-reduced-motion: reduce) { body { animation: none; } }
"#;

/// Only a plain host name is shown; anything else reads "that site".
fn shown_site(site: &str) -> Option<&str> {
    let ok = !site.is_empty() && site.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    ok.then_some(site)
}

/// The whole page for a tab that was showing `site`, lit for `now`.
pub fn page(site: &str, now: DateTime<Local>) -> String {
    let hour = now.hour() as f64 + now.minute() as f64 / 60.0;
    let s = forest::sky(hour, &forest::CALM);
    let far_fill = forest::mix(&s.horizon, &s.near, 0.5);
    let (ink, muted, site_ink, lift) = if s.day {
        ("#10221a", "#24362c", "#2a6644", "0 0 22px rgba(236, 242, 232, 0.55)")
    } else {
        ("#e8f0e5", "#cfdacb", "#cfe2a4", "0 1px 14px rgba(0, 8, 6, 0.45)")
    };
    // one fixed forest per day
    let seed = forest::day_seed(now.year(), now.month0(), now.day(), 7);
    let line = |base: f64, min_h: f64, max_h: f64, min_w: f64, max_w: f64, seed: u32, hills: f64| {
        forest::treeline(&forest::Treeline {
            width: W,
            height: H,
            base: H * base,
            min_h: H * min_h,
            max_h: H * max_h,
            min_w: H * min_w,
            max_w: H * max_w,
            seed,
            hills: H * hills,
            ..Default::default()
        })
    };
    let far = line(0.82, 0.12, 0.45, 0.07, 0.13, seed, 0.05);
    let near = line(1.0, 0.05, 0.62, 0.15, 0.26, seed.wrapping_add(131), 0.02);
    let (title, name) = match shown_site(site) {
        Some(site) => (format!("{site} closed by Defixum"), site),
        None => ("Tab closed by Defixum".to_string(), "that site"),
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ --sky-top: {top}; --horizon: {horizon}; --ink: {ink}; --muted: {muted}; --site: {site_ink}; --lift: {lift}; }}{STYLE}</style>
</head>
<body>
<svg class="trees" aria-hidden="true" viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMax slice">
<path fill="{far_fill}" d="{far}"/>
<path fill="{near_fill}" d="{near}"/>
</svg>
<main>
<h1>You closed <span class="site">{name}</span>.</h1>
<p>This tab is free for whatever you meant to do.</p>
</main>
</body>
</html>
"#,
        top = s.top,
        horizon = s.horizon,
        near_fill = s.near,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn names_only_plain_hosts() {
        let now = Local.with_ymd_and_hms(2026, 11, 16, 22, 30, 0).unwrap();
        let p = page("youtube.com", now);
        assert!(p.contains("<span class=\"site\">youtube.com</span>"));
        assert!(p.contains("<title>youtube.com closed by Defixum</title>"));
        assert!(!p.contains("<script"));
        let p = page("<b>x</b>", now);
        assert!(p.contains(">that site<") && !p.contains("<b>"));
    }

    #[test]
    fn same_forest_all_day() {
        let a = page("a.com", Local.with_ymd_and_hms(2026, 11, 16, 9, 0, 0).unwrap());
        let b = page("a.com", Local.with_ymd_and_hms(2026, 11, 16, 9, 0, 0).unwrap());
        let c = page("a.com", Local.with_ymd_and_hms(2026, 11, 17, 9, 0, 0).unwrap());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
