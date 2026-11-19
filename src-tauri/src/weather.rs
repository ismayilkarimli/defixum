//! The weather outside, for the pause screen's sky. Location is approximate: the city an IP
//! lookup places this Mac in (ipwho.is, then geojs.io), or failing that the time zone's main
//! city from the system zone table. Weather comes from Open-Meteo. None of them need a key.
//! Requests go through the system `curl`, so there is no HTTP stack to bundle; offline, the
//! last reading simply ages out and the sky goes back to fair weather.

use serde::Serialize;
use serde_json::Value;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Debug)]
pub struct Weather {
    /// WMO weather interpretation code (0 clear … 99 thunderstorm with hail)
    pub code: u32,
    /// percent of the sky covered
    pub cloud: f64,
    /// km/h at 10 m
    pub wind: f64,
    /// mm over the last quarter hour
    pub precip: f64,
    /// today's sunrise and sunset, unix seconds
    pub sunrise: i64,
    pub sunset: i64,
    /// unix seconds the reading was taken
    pub at: i64,
}

struct Place {
    lat: f64,
    lon: f64,
    at: SystemTime,
}

static PLACE: Mutex<Option<Place>> = Mutex::new(None);
static NOW: Mutex<Option<Weather>> = Mutex::new(None);
static LAST_TRY: Mutex<Option<SystemTime>> = Mutex::new(None);

const FRESH: Duration = Duration::from_secs(20 * 60);
const RETRY: Duration = Duration::from_secs(5 * 60);
const PLACE_FRESH: Duration = Duration::from_secs(6 * 3600);
/// Older than this, a reading no longer describes the sky outside.
const STALE: Duration = Duration::from_secs(3 * 3600);

fn age(t: SystemTime) -> Duration {
    SystemTime::now().duration_since(t).unwrap_or_default()
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The latest reading, if it is recent enough to trust.
pub fn current() -> Option<Weather> {
    let w = NOW.lock().unwrap().clone()?;
    (unix_now() - w.at < STALE.as_secs() as i64).then_some(w)
}

/// Fetches a new reading when the last one is old. Cheap to call often; meant for a background
/// thread, since a fetch blocks for up to a few seconds.
pub fn refresh_if_due() {
    let fresh = current().is_some_and(|w| unix_now() - w.at < FRESH.as_secs() as i64);
    let tried = LAST_TRY.lock().unwrap().is_some_and(|t| age(t) < RETRY);
    if fresh || tried {
        return;
    }
    *LAST_TRY.lock().unwrap() = Some(SystemTime::now());
    let Some((lat, lon)) = place() else { return };
    match fetch(lat, lon) {
        Some(w) => {
            crate::log(format!("weather: code {} cloud {}% wind {} km/h", w.code, w.cloud, w.wind));
            *NOW.lock().unwrap() = Some(w);
        }
        None => crate::log("weather: no reading (offline?)"),
    }
}

fn get(url: &str) -> Option<Value> {
    let out = Command::new("/usr/bin/curl")
        .args(["-sfL", "--max-time", "10", "-A", "Defixum", url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.parse().ok())
}

fn place() -> Option<(f64, f64)> {
    if let Some(p) = PLACE.lock().unwrap().as_ref().filter(|p| age(p.at) < PLACE_FRESH) {
        return Some((p.lat, p.lon));
    }
    let looked_up = get("https://ipwho.is/?fields=success,latitude,longitude")
        .filter(|v| v["success"].as_bool() == Some(true))
        .and_then(|v| Some((num(&v["latitude"])?, num(&v["longitude"])?)))
        .or_else(|| {
            let v = get("https://get.geojs.io/v1/ip/geo.json")?;
            Some((num(&v["latitude"])?, num(&v["longitude"])?))
        });
    let found = looked_up.or_else(zone_place)?;
    // a city is plenty: round to about a kilometre before it goes anywhere else
    let (lat, lon) = ((found.0 * 100.0).round() / 100.0, (found.1 * 100.0).round() / 100.0);
    // only a real lookup is worth keeping for hours; the zone guess is retried next time
    if looked_up.is_some() {
        *PLACE.lock().unwrap() = Some(Place { lat, lon, at: SystemTime::now() });
    }
    Some((lat, lon))
}

/// The main city of this Mac's time zone, from the system zone table (no network).
fn zone_place() -> Option<(f64, f64)> {
    let link = std::fs::read_link("/etc/localtime").ok()?;
    let link = link.to_string_lossy();
    let zone = &link[link.find("zoneinfo/")? + "zoneinfo/".len()..];
    let table = std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()?;
    let line = table.lines().find(|l| !l.starts_with('#') && l.split('\t').nth(2) == Some(zone))?;
    parse_iso6709(line.split('\t').nth(1)?)
}

/// "+5925+02445" or "-332552-0703646": ±DDMM[SS] latitude then ±DDDMM[SS] longitude.
fn parse_iso6709(s: &str) -> Option<(f64, f64)> {
    let split = s[1..].find(['+', '-'])? + 1;
    let part = |p: &str, deg_digits: usize| -> Option<f64> {
        let sign = if p.starts_with('-') { -1.0 } else { 1.0 };
        let d = &p[1..];
        let deg: f64 = d.get(..deg_digits)?.parse().ok()?;
        let min: f64 = d.get(deg_digits..deg_digits + 2)?.parse().ok()?;
        let sec: f64 = d.get(deg_digits + 2..deg_digits + 4).and_then(|x| x.parse().ok()).unwrap_or(0.0);
        Some(sign * (deg + min / 60.0 + sec / 3600.0))
    };
    Some((part(&s[..split], 2)?, part(&s[split..], 3)?))
}

fn fetch(lat: f64, lon: f64) -> Option<Weather> {
    let v = get(&format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=weather_code,cloud_cover,precipitation,wind_speed_10m\
         &daily=sunrise,sunset&timezone=auto&timeformat=unixtime&forecast_days=1"
    ))?;
    let c = &v["current"];
    Some(Weather {
        code: c["weather_code"].as_u64()? as u32,
        cloud: c["cloud_cover"].as_f64().unwrap_or(0.0),
        wind: c["wind_speed_10m"].as_f64().unwrap_or(0.0),
        precip: c["precipitation"].as_f64().unwrap_or(0.0),
        sunrise: v["daily"]["sunrise"][0].as_i64().unwrap_or(0),
        sunset: v["daily"]["sunset"][0].as_i64().unwrap_or(0),
        at: unix_now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_table_coordinates() {
        let (lat, lon) = parse_iso6709("+5925+02445").unwrap();
        assert!((lat - 59.4167).abs() < 0.001 && (lon - 24.75).abs() < 0.001);
        let (lat, lon) = parse_iso6709("-332552-0703646").unwrap();
        assert!((lat + 33.4311).abs() < 0.001 && (lon + 70.6128).abs() < 0.001);
    }
}
