//! The weather reading and the hour the sky is drawn for, in the page's own time zone.

use serde::Deserialize;

use crate::dom;

/// A reading from the app (see weather.rs there).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct Reading {
    pub code: i64,
    pub cloud: f64,
    pub wind: f64,
    /// today's sunrise and sunset, unix seconds (0 when unknown)
    pub sunrise: i64,
    pub sunset: i64,
}

impl Default for Reading {
    fn default() -> Self {
        Reading { code: 0, cloud: 0.0, wind: 0.0, sunrise: 0, sunset: 0 }
    }
}

/// What the sky does for a reading, plus whether there was one.
pub fn weather(r: Option<&Reading>) -> forest::Weather {
    forest::weather(r.map(|r| (r.code, r.cloud, r.wind)))
}

/// The hour on this Mac's clock (`?hour=` overrides it for design work).
pub fn now_hour() -> f64 {
    if let Some(h) = dom::param("hour").filter(|h| !h.is_empty()) {
        return h.parse().unwrap_or(12.0);
    }
    let d = js_sys::Date::new_0();
    d.get_hours() as f64 + d.get_minutes() as f64 / 60.0
}

fn clock_hour(unix: i64) -> f64 {
    let d = js_sys::Date::new(&((unix as f64) * 1000.0).into());
    d.get_hours() as f64 + d.get_minutes() as f64 / 60.0
}

/// The clock hour, stretched so dawn and dusk land on the real sunrise and sunset outside.
pub fn solar_hour(hour: f64, r: Option<&Reading>) -> f64 {
    let sun = r.filter(|r| r.sunrise != 0 && r.sunset != 0).map(|r| (clock_hour(r.sunrise), clock_hour(r.sunset)));
    forest::solar_hour(hour, sun)
}
