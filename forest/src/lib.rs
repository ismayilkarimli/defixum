//! Procedural conifer tree lines and the sky for the hour, shared by the pause screen, the
//! settings window and the page left behind when a tab can't be closed.

/// Small seeded generator (mulberry32), so a forest stays the same for a whole day.
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed)
    }

    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}

/// The seed for a day's forest: the same all day, different the next.
pub fn day_seed(year: i32, month0: u32, day: u32, salt: u32) -> u32 {
    ((year as i64 * 400 + month0 as i64 * 32 + day as i64) * 97 + salt as i64) as u32
}

/// One spruce: drooping tiers that narrow to a spire. Tiers alternate an outer branch tip with
/// an inner notch, so the silhouette reads as a conifer rather than a triangle.
fn spruce(x: f64, base: f64, w: f64, h: f64, r: &mut Rng) -> String {
    let tiers = 5 + (r.next() * 4.0).floor() as usize;
    let lean = (r.next() - 0.5) * w * 0.08;
    let mut left = Vec::with_capacity(tiers * 2);
    for i in 0..tiers {
        let t = i as f64 / tiers as f64;
        let y = base - h * t * 0.94;
        let half = (w / 2.0) * (1.0 - t).powf(0.85) * (0.9 + r.next() * 0.2);
        let notch_y = y - (h * 0.94) / tiers as f64 * (0.55 + r.next() * 0.2);
        left.push((x - half + lean * t, y));
        left.push((x - half * 0.32 + lean * t, notch_y));
    }
    let apex = (x + lean, base - h);
    let right: Vec<(f64, f64)> =
        left.iter().rev().map(|&(px, py)| (2.0 * (x + lean * ((base - py) / h)) - px, py)).collect();
    let mut pts = vec![(x - w * 0.05, base + 2.0)];
    pts.extend(left);
    pts.push(apex);
    pts.extend(right);
    pts.push((x + w * 0.05, base + 2.0));
    let mut d = String::from("M");
    for (n, (px, py)) in pts.iter().enumerate() {
        if n > 0 {
            d.push('L');
        }
        d.push_str(&format!("{px:.1} {py:.1}"));
    }
    d.push('Z');
    d
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Lowers the trees around a centre point so text there stays readable.
#[derive(Clone, Copy)]
pub struct Clearing {
    pub center: f64,
    pub half: f64,
    pub floor: f64,
}

/// A band of overlapping spruces standing on `base`, filled solid down to `height`.
#[derive(Clone, Copy)]
pub struct Treeline {
    pub width: f64,
    pub height: f64,
    pub base: f64,
    pub min_h: f64,
    pub max_h: f64,
    pub min_w: f64,
    pub max_w: f64,
    pub seed: u32,
    pub clearing: Option<Clearing>,
    pub hills: f64,
    pub scrub: f64,
    pub spacing: (f64, f64),
}

impl Default for Treeline {
    fn default() -> Self {
        Treeline {
            width: 0.0,
            height: 0.0,
            base: 0.0,
            min_h: 0.0,
            max_h: 0.0,
            min_w: 0.0,
            max_w: 0.0,
            seed: 0,
            clearing: None,
            hills: 0.0,
            scrub: 0.0,
            spacing: (0.32, 0.72),
        }
    }
}

/// The SVG path of a tree line.
pub fn treeline(o: &Treeline) -> String {
    let mut r = Rng::new(o.seed);
    let width = o.width;
    // the ground rolls: two slow waves, so far ridges read as hills rather than a ruler line
    let f1 = (2.0 + r.next() * 2.0) / width;
    let f2 = (5.0 + r.next() * 4.0) / width;
    let p1 = r.next() * 7.0;
    let p2 = r.next() * 7.0;
    let pi = std::f64::consts::PI;
    let ground = |x: f64| o.base - o.hills * (0.6 * (x * f1 * pi + p1).sin() + 0.4 * (x * f2 * pi + p2).sin());
    let mut d = format!("M0 {}", num(o.height));
    if o.scrub > 0.0 {
        // undergrowth: low rounded tufts along the ground
        let mut x = 0.0;
        while x <= width {
            let w = 6.0 + r.next() * 14.0;
            let top = ground(x + w / 2.0) - o.scrub * (0.25 + r.next() * 0.75);
            d.push_str(&format!(
                "L{:.1} {:.1}Q{:.1} {:.1} {:.1} {:.1}",
                x,
                ground(x),
                x + w / 2.0,
                2.0 * top - ground(x),
                x + w,
                ground(x + w)
            ));
            x += w;
        }
    } else {
        let mut x = 0.0;
        while x <= width {
            d.push_str(&format!("L{:.1} {:.1}", x, ground(x)));
            x += width / 48.0;
        }
    }
    d.push_str(&format!("L{} {:.1}L{} {}Z", num(width), ground(width), num(width), num(o.height)));
    let mut x = -o.max_w * r.next();
    while x < width + o.max_w {
        let w = o.min_w + (o.max_w - o.min_w) * r.next();
        let mut h = o.min_h + (o.max_h - o.min_h) * r.next().powf(1.6);
        if let Some(c) = o.clearing {
            let dist = ((x - c.center).abs() / c.half).min(1.0);
            h *= c.floor + (1.0 - c.floor) * smooth(dist);
        }
        if h > 1.0 {
            let b = ground(x) + r.next() * o.min_h * 0.15;
            d.push_str(&spruce(x, b, w, h, &mut r));
        }
        x += w * (o.spacing.0 + r.next() * (o.spacing.1 - o.spacing.0));
    }
    d
}

/// A number as JavaScript would print it (no trailing ".0").
pub fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

// ---------- colour ----------

fn hex(h: &str) -> [f64; 3] {
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f64;
    [p(1), p(3), p(5)]
}

fn to_hex(c: [f64; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", js_round(c[0]) as u8, js_round(c[1]) as u8, js_round(c[2]) as u8)
}

fn js_round(v: f64) -> f64 {
    (v + 0.5).floor().clamp(0.0, 255.0)
}

/// `a` moved towards `b` by `t` (0–1), as "#rrggbb".
pub fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (hex(a), hex(b));
    to_hex([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t])
}

fn luminance(h: &str) -> f64 {
    let c = hex(h).map(|v| {
        let v = v / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The sky over the trees through the day: (hour, top, horizon). Interpolated, so the light
/// drifts with the clock instead of switching between themes.
const SKY: [(f64, &str, &str); 10] = [
    (0.0, "#07131a", "#22403c"),
    (4.5, "#0b1b22", "#2a4542"),
    (6.5, "#8aa3a9", "#d6dccf"),
    (9.5, "#a5c0bf", "#dfe8de"),
    (16.0, "#a0bcb9", "#dce5d8"),
    (17.4, "#a9b7a4", "#e3d6a6"),
    (18.5, "#c4ad80", "#e8c48a"),
    (20.25, "#20364a", "#5c6e69"),
    (22.0, "#07131a", "#22403c"),
    (24.0, "#07131a", "#22403c"),
];

/// A grey of the same brightness, for skies under cloud.
fn grey(h: &str) -> String {
    let [r, g, b] = hex(h);
    let v = 0.3 * r + 0.55 * g + 0.15 * b;
    to_hex([(v * 0.98).min(255.0), v.min(255.0), (v * 1.01).min(255.0)])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sky {
    pub top: String,
    pub horizon: String,
    pub near: String,
    pub day: bool,
    pub haze: f64,
    pub stars: f64,
}

pub fn sky(hour: f64, wx: &Weather) -> Sky {
    let mut i = 0;
    while i < SKY.len() - 2 && hour >= SKY[i + 1].0 {
        i += 1;
    }
    let (h0, t0, b0) = SKY[i];
    let (h1, t1, b1) = SKY[i + 1];
    let t = ((hour - h0) / (h1 - h0)).clamp(0.0, 1.0);
    let mut top = mix(t0, t1, t);
    let mut horizon = mix(b0, b1, t);
    // weather: cloud drains the colour and dims the light; fog flattens the sky into one pale wash
    if wx.gloom > 0.0 {
        // storm light is dark and heavy; a snowy sky goes pale instead
        let snow = wx.snow > 0.0;
        let dim = if snow { "#9aa3a3" } else { "#26302e" };
        let lift = wx.gloom * 0.32 + if wx.storm { 0.3 } else { 0.0 };
        top = mix(&mix(&top, &grey(&top), wx.gloom), dim, lift);
        horizon = mix(&mix(&horizon, &grey(&horizon), wx.gloom), if snow { "#b4bcbb" } else { "#3a4442" }, lift * 0.6);
    }
    if wx.fog > 0.0 {
        top = mix(&top, &horizon, wx.fog * 0.65);
    }
    let light = luminance(&mix(&top, &horizon, 0.5));
    // nearest trees: near-black spruce, a touch greener in daylight
    let near = mix("#020806", "#0a1c15", (light / 0.4).min(1.0));
    // Words sit about 60% of the way down the sky. Dark ink and light ink read equally well at a
    // luminance near 0.2, so that is where they swap; close to the swap (dawn, dusk) neither has
    // much contrast, so a haze behind the words thickens to push the sky away from the ink.
    let behind = luminance(&mix(&top, &horizon, 0.6));
    let haze = 0.08 + 0.4 * (1.0 - (behind - 0.2).abs() / 0.18).max(0.0);
    let dark = ((0.08 - light) / 0.05).clamp(0.0, 1.0);
    // cloud hides the stars; a thin veil still lets most through
    let stars = dark * (1.0 - wx.cover.powf(1.6));
    Sky { top, horizon, near, day: behind > 0.2, haze, stars }
}

// ---------- weather ----------

/// What the sky does for a weather reading. Strengths are 0–1.
#[derive(Clone, Debug, PartialEq)]
pub struct Weather {
    pub kind: &'static str,
    pub cover: f64,
    pub gloom: f64,
    pub fog: f64,
    pub rain: f64,
    pub snow: f64,
    pub storm: bool,
    pub wind: f64,
}

pub const CALM: Weather =
    Weather { kind: "clear", cover: 0.0, gloom: 0.0, fog: 0.0, rain: 0.0, snow: 0.0, storm: false, wind: 0.0 };

/// Rain, snow and fog strengths for each WMO weather code Open-Meteo reports.
fn code(c: i64) -> Option<(&'static str, f64, f64, f64, bool)> {
    // (kind, fog, rain, snow, storm)
    Some(match c {
        45 | 48 => ("fog", 1.0, 0.0, 0.0, false),
        51 => ("drizzle", 0.0, 0.2, 0.0, false),
        53 => ("drizzle", 0.0, 0.3, 0.0, false),
        55 => ("drizzle", 0.0, 0.4, 0.0, false),
        56 => ("drizzle", 0.0, 0.3, 0.0, false),
        57 => ("drizzle", 0.0, 0.45, 0.0, false),
        61 => ("rain", 0.0, 0.45, 0.0, false),
        63 => ("rain", 0.0, 0.7, 0.0, false),
        65 => ("rain", 0.0, 1.0, 0.0, false),
        66 => ("rain", 0.0, 0.5, 0.0, false),
        67 => ("rain", 0.0, 0.9, 0.0, false),
        80 => ("rain", 0.0, 0.5, 0.0, false),
        81 => ("rain", 0.0, 0.75, 0.0, false),
        82 => ("rain", 0.0, 1.0, 0.0, false),
        71 => ("snow", 0.0, 0.0, 0.35, false),
        73 => ("snow", 0.0, 0.0, 0.65, false),
        75 => ("snow", 0.0, 0.0, 1.0, false),
        77 => ("snow", 0.0, 0.0, 0.25, false),
        85 => ("snow", 0.0, 0.0, 0.6, false),
        86 => ("snow", 0.0, 0.0, 1.0, false),
        95 => ("storm", 0.0, 0.85, 0.0, true),
        96 | 99 => ("storm", 0.0, 1.0, 0.0, true),
        _ => return None,
    })
}

/// What the sky should do for a reading (weather code, cloud %, wind km/h); None is fair weather.
pub fn weather(reading: Option<(i64, f64, f64)>) -> Weather {
    let Some((c, cloud, wind)) = reading else { return CALM };
    let mut w = CALM;
    match code(c) {
        Some((kind, fog, rain, snow, storm)) => {
            w.kind = kind;
            w.fog = fog;
            w.rain = rain;
            w.snow = snow;
            w.storm = storm;
        }
        None => w.kind = if c >= 3 { "overcast" } else if c == 2 { "cloudy" } else { "clear" },
    }
    // falling weather comes from a full sky, whatever the cloud figure says
    let wet = w.rain.max(w.snow);
    w.cover = (cloud / 100.0)
        .min(1.0)
        .max(if w.kind == "overcast" { 0.9 } else { 0.0 })
        .max(if wet > 0.0 { 0.9 } else { 0.0 })
        .max(w.fog);
    w.gloom = (((w.cover - 0.55) / 0.45).max(0.0) * 0.5
        + w.rain * 0.25
        + w.snow * 0.1
        + if w.storm { 0.2 } else { 0.0 }
        + w.fog * 0.3)
        .min(0.9);
    w.wind = (wind / 45.0).min(1.0);
    w
}

/// Where the clock hour falls on the sky's own day, whose sun rises at 6.6 and sets at 18.6, so
/// the real sunrise and sunset (hours on the local clock) land on dawn and dusk whatever the season.
pub fn solar_hour(hour: f64, sun: Option<(f64, f64)>) -> f64 {
    let Some((rise, set)) = sun else { return hour };
    let day = set - rise;
    if !(day > 1.0 && day < 23.0) {
        return hour;
    }
    if hour >= rise && hour <= set {
        return 6.6 + ((hour - rise) / day) * 12.0;
    }
    (18.6 + (((hour - set + 24.0) % 24.0) / (24.0 - day)) * 12.0) % 24.0
}

/// How strongly golden-hour light is in the air: 1 at sunset or sunrise, 0 away from them.
pub fn golden(hour: f64) -> f64 {
    0f64.max(1.0 - (hour - 18.6).abs() / 1.3).max(1.0 - (hour - 6.6).abs() / 1.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_matches_mulberry32() {
        // first outputs of mulberry32(1) in JavaScript
        let mut r = Rng::new(1);
        assert!((r.next() - 0.6270739405881613).abs() < 1e-12);
        assert!((r.next() - 0.002735721180215478).abs() < 1e-12);
    }

    #[test]
    fn mixes_colours() {
        assert_eq!(mix("#000000", "#ffffff", 0.5), "#808080");
        assert_eq!(mix("#07131a", "#22403c", 0.0), "#07131a");
    }

    #[test]
    fn night_and_day() {
        assert!(!sky(23.5, &CALM).day);
        assert!(sky(11.0, &CALM).day);
        assert!(sky(23.5, &CALM).stars > 0.9);
    }
}
