//! The weather outside, for the pause screen's sky. Location is approximate: the city an IP
//! lookup places this Mac in (ipwho.is, then geojs.io), or failing that the time zone's main
//! city from the system zone table. Weather comes from Open-Meteo. None of them need a key.
//! The same IP lookup names the country (for the scenery), trusted only when its time zone
//! keeps this Mac's clock; otherwise (a VPN) or offline, the country comes from the time zone,
//! and last from the Region setting. No permission is asked for any of it.
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
/// the country the last IP lookup placed this Mac in, when its time zone agreed with the clock
static COUNTRY: Mutex<Option<(String, SystemTime)>> = Mutex::new(None);
static LAST_LOCATE: Mutex<Option<SystemTime>> = Mutex::new(None);
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
    let offset = chrono::Local::now().offset().local_minus_utc() as i64;
    let zone = local_zone();
    let looked_up = get("https://ipwho.is/?fields=success,latitude,longitude,country_code,timezone")
        .filter(|v| v["success"].as_bool() == Some(true))
        .and_then(|v| {
            let agrees = v["timezone"]["offset"].as_i64() == Some(offset);
            Some((num(&v["latitude"])?, num(&v["longitude"])?, country_code(&v).filter(|_| agrees)))
        })
        .or_else(|| {
            let v = get("https://get.geojs.io/v1/ip/geo.json")?;
            let agrees = zone.is_some() && v["timezone"].as_str() == zone.as_deref();
            Some((num(&v["latitude"])?, num(&v["longitude"])?, country_code(&v).filter(|_| agrees)))
        })
        .map(|(lat, lon, country)| {
            *COUNTRY.lock().unwrap() = country.map(|c| (c, SystemTime::now()));
            (lat, lon)
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

fn country_code(v: &Value) -> Option<String> {
    let c = v["country_code"].as_str()?.trim().to_uppercase();
    (c.len() == 2 && c.chars().all(|ch| ch.is_ascii_uppercase())).then_some(c)
}

/// This Mac's time zone, as "Europe/Tallinn".
fn local_zone() -> Option<String> {
    let link = std::fs::read_link("/etc/localtime").ok()?;
    let link = link.to_string_lossy();
    Some(link[link.find("zoneinfo/")? + "zoneinfo/".len()..].to_string())
}

/// The time zone's row in the system zone table: country code, coordinates, zone.
fn zone_row() -> Option<Vec<String>> {
    let zone = local_zone()?;
    let table = std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()?;
    let line = table.lines().find(|l| !l.starts_with('#') && l.split('\t').nth(2) == Some(zone.as_str()))?;
    Some(line.split('\t').map(str::to_string).collect())
}

/// The main city of this Mac's time zone, from the system zone table (no network).
fn zone_place() -> Option<(f64, f64)> {
    parse_iso6709(zone_row()?.get(1)?)
}

/// Looks this Mac up again when the last lookup is old. Runs whatever the weather setting, for
/// the country; offline it waits a few minutes between tries.
pub fn locate_if_due() {
    if PLACE.lock().unwrap().as_ref().is_some_and(|p| age(p.at) < PLACE_FRESH) {
        return;
    }
    if LAST_LOCATE.lock().unwrap().is_some_and(|t| age(t) < RETRY) {
        return;
    }
    *LAST_LOCATE.lock().unwrap() = Some(SystemTime::now());
    let _ = place();
}

/// The country this Mac is in, as an ISO code ("EE"): the IP lookup when its time zone agrees
/// with the clock, else the time zone's country, else the Region setting.
pub fn country() -> Option<String> {
    if let Some((c, at)) = COUNTRY.lock().unwrap().clone() {
        if age(at) < PLACE_FRESH {
            return Some(c);
        }
    }
    zone_row().and_then(|r| r.first().cloned()).filter(|c| c.len() == 2).or_else(region)
}

/// The Region setting's country ("en_EE" → "EE").
fn region() -> Option<String> {
    let out = Command::new("/usr/bin/defaults").args(["read", "-g", "AppleLocale"]).output().ok()?;
    let locale = String::from_utf8_lossy(&out.stdout);
    let c = locale.trim().split(['_', '@']).nth(1)?.to_uppercase();
    (c.len() == 2 && c.chars().all(|ch| ch.is_ascii_uppercase())).then_some(c)
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

#[cfg(test)]
mod country_tests {
    /// Prints what the time zone and Region setting say here; run with --ignored --nocapture.
    #[test]
    #[ignore]
    fn country_here() {
        println!("zone {:?}, country {:?}, region {:?}", super::local_zone(), super::country(), super::region());
    }
}
