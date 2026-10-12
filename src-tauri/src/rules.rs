//! Rule model and the pure decision logic: does this URL, right now, warrant a pause or a block?

use chrono::{DateTime, Datelike, Local, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Pause,
    Block,
}

/// How the pause countdown is drawn on the overlay.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Countdown {
    /// A circle that grows and shrinks with the breath.
    #[default]
    Breathe,
    /// A ring that fills clockwise.
    Fill,
    /// A ring that empties back to 12 o'clock.
    Drain,
}

/// A kind of message the pause screen can show instead of a fixed headline.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptKind {
    /// A mindful question while you wait.
    Reflect,
    /// What this visit takes from you.
    Cost,
    /// Type out an admission before the site opens.
    Commit,
}

/// When a rule applies. Disabled means "always". `days[0]` is Monday.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Schedule {
    pub enabled: bool,
    pub days: [bool; 7],
    pub start: String,
    pub end: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule {
            enabled: false,
            days: [true, true, true, true, true, false, false],
            start: "09:00".into(),
            end: "17:00".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub sites: Vec<String>,
    pub mode: Mode,
    pub pause_seconds: u32,
    pub countdown: Countdown,
    /// Message kinds to pick from at random. Empty behaves like `[Reflect]`.
    pub prompts: Vec<PromptKind>,
    pub daily_opens: Option<u32>,
    pub daily_minutes: Option<u32>,
    pub schedule: Schedule,
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            id: String::new(),
            name: "New rule".into(),
            enabled: true,
            sites: vec![],
            mode: Mode::Pause,
            pause_seconds: 10,
            countdown: Countdown::Breathe,
            prompts: vec![PromptKind::Reflect],
            daily_opens: None,
            daily_minutes: None,
            schedule: Schedule::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Minutes offered on the overlay's "continue for…" buttons.
    pub session_options: Vec<u32>,
    /// Stop counting time on a site after this many seconds without input.
    pub idle_seconds: u64,
    /// How the pause screen and settings window are lit: "time" (the sky of the local hour),
    /// "light" or "dark".
    pub sky: String,
    /// Rain, snow, fog and cloud on the pause screen follow the weather outside.
    pub weather: bool,
    /// Full-screen reminders before the sleep time (see `bedtime`). Off until asked for.
    pub bedtime: bool,
    /// The sleep time, "HH:MM" on a 24-hour clock.
    pub bedtime_at: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            session_options: vec![2, 5, 15],
            idle_seconds: 60,
            sky: "time".into(),
            weather: true,
            bedtime: false,
            bedtime_at: "23:00".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Config {
    pub rules: Vec<Rule>,
    pub settings: Settings,
}

impl Config {
    pub fn starter() -> Self {
        Config {
            rules: vec![Rule {
                id: "starter".into(),
                name: "Social & video".into(),
                sites: ["youtube.com", "reddit.com", "x.com", "instagram.com"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                ..Rule::default()
            }],
            settings: Settings::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct DayStats {
    /// Times the pause screen appeared.
    pub intercepts: u32,
    /// Times the user chose to continue.
    pub opens: u32,
    /// Times the user turned back.
    pub skips: u32,
    /// Seconds spent on the rule's sites (while allowed and not idle).
    pub seconds: u64,
}

/// date ("YYYY-MM-DD") → rule id → counters
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Stats {
    pub days: BTreeMap<String, HashMap<String, DayStats>>,
}

impl Stats {
    pub fn day(&self, date: &str, rule: &str) -> DayStats {
        self.days.get(date).and_then(|d| d.get(rule)).copied().unwrap_or_default()
    }

    pub fn day_mut(&mut self, date: &str, rule: &str) -> &mut DayStats {
        self.days.entry(date.to_string()).or_default().entry(rule.to_string()).or_default()
    }

    pub fn prune(&mut self, keep_days: usize) {
        while self.days.len() > keep_days {
            let first = self.days.keys().next().cloned().unwrap();
            self.days.remove(&first);
        }
    }
}

pub fn date_key(now: &DateTime<Local>) -> String {
    now.format("%Y-%m-%d").to_string()
}

// ---------- URL matching ----------

/// Splits a URL (or bare "host/path" as shown in some address bars) into (host, path).
/// Returns None for non-web schemes such as about:, file:, chrome:, moz-extension:.
pub fn split_url(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let rest = match url.find("://") {
        Some(i) => {
            let scheme = url[..i].to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                return None;
            }
            &url[i + 3..]
        }
        None => {
            if url.contains(':') && !url.split('/').next().unwrap_or("").contains('.') {
                return None; // about:blank, chrome:newtab …
            }
            url
        }
    };
    let (authority, path) = match rest.find(|c| c == '/' || c == '?' || c == '#') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    let host = host.trim_start_matches("www.").trim_end_matches('.').to_string();
    if host.is_empty() {
        return None;
    }
    Some((host, path.to_string()))
}

/// A site pattern is "domain" or "domain/path-prefix". Subdomains match: "youtube.com"
/// covers "m.youtube.com".
pub fn site_matches(pattern: &str, host: &str, path: &str) -> bool {
    let Some((p_host, p_path)) = split_url(pattern) else { return false };
    let host_ok = host == p_host || host.ends_with(&format!(".{p_host}"));
    if !host_ok {
        return false;
    }
    let p_path = p_path.trim_end_matches('/');
    p_path.is_empty() || path.to_ascii_lowercase().starts_with(&p_path.to_ascii_lowercase())
}

pub fn rule_matches(rule: &Rule, url: &str) -> bool {
    let Some((host, path)) = split_url(url) else { return false };
    rule.sites.iter().any(|s| !s.trim().is_empty() && site_matches(s, &host, &path))
}

pub fn host_of(url: &str) -> String {
    split_url(url).map(|(h, _)| h).unwrap_or_default()
}

// ---------- schedules ----------

fn parse_hm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

pub fn schedule_active(s: &Schedule, now: &DateTime<Local>) -> bool {
    if !s.enabled {
        return true;
    }
    let (Some(start), Some(end)) = (parse_hm(&s.start), parse_hm(&s.end)) else { return true };
    let wd = now.weekday().num_days_from_monday() as usize;
    let prev = (wd + 6) % 7;
    let t = NaiveTime::from_hms_opt(now.hour(), now.minute(), now.second()).unwrap();
    if start == end {
        s.days[wd]
    } else if start < end {
        s.days[wd] && t >= start && t < end
    } else {
        // overnight window, e.g. 22:00–02:00: the early part belongs to the previous day
        (s.days[wd] && t >= start) || (s.days[prev] && t < end)
    }
}

// ---------- verdicts ----------

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Normal mindful pause; the user may continue after the countdown.
    Pause,
    /// The previous "continue" window ran out while still on the site.
    Expired,
    /// A block-mode rule is in effect.
    Blocked,
    OpenLimit,
    TimeLimit,
}

impl Reason {
    pub fn can_continue(self) -> bool {
        matches!(self, Reason::Pause | Reason::Expired)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub rule_index: usize,
    /// None: the user is inside an allowance window (or nothing applies yet).
    pub reason: Option<Reason>,
}

/// Picks the rule that governs `url` now. Block rules win over pause rules; among pause
/// rules the first in list order wins. Limits turn a pause into a block.
pub fn evaluate(
    config: &Config,
    stats: &Stats,
    now: &DateTime<Local>,
    url: &str,
    allowed: &dyn Fn(&str) -> bool,
) -> Option<Verdict> {
    let today = date_key(now);
    let active: Vec<usize> = config
        .rules
        .iter()
        .enumerate()
        .filter(|(_, r)| r.enabled && schedule_active(&r.schedule, now) && rule_matches(r, url))
        .map(|(i, _)| i)
        .collect();
    if let Some(&i) = active.iter().find(|&&i| config.rules[i].mode == Mode::Block) {
        return Some(Verdict { rule_index: i, reason: Some(Reason::Blocked) });
    }
    let &i = active.first()?;
    let rule = &config.rules[i];
    let day = stats.day(&today, &rule.id);
    let reason = if rule.daily_minutes.is_some_and(|m| day.seconds >= m as u64 * 60) {
        Some(Reason::TimeLimit)
    } else if allowed(&rule.id) {
        None
    } else if rule.daily_opens.is_some_and(|n| day.opens >= n) {
        Some(Reason::OpenLimit)
    } else {
        Some(Reason::Pause)
    };
    Some(Verdict { rule_index: i, reason })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn splits_urls() {
        assert_eq!(split_url("https://www.YouTube.com/watch?v=1"), Some(("youtube.com".into(), "/watch?v=1".into())));
        assert_eq!(split_url("m.reddit.com/r/rust"), Some(("m.reddit.com".into(), "/r/rust".into())));
        assert_eq!(split_url("http://user@x.com:8080"), Some(("x.com".into(), "".into())));
        assert_eq!(split_url("about:blank"), None);
        assert_eq!(split_url("file:///Users/a/calm.html"), None);
        assert_eq!(split_url("moz-extension://abc/page.html"), None);
        assert_eq!(split_url("localhost:3000/x"), None);
    }

    #[test]
    fn matches_sites() {
        let r = Rule { sites: vec!["youtube.com".into(), "reddit.com/r/all".into()], ..Rule::default() };
        assert!(rule_matches(&r, "https://www.youtube.com/"));
        assert!(rule_matches(&r, "https://m.youtube.com/shorts/x"));
        assert!(!rule_matches(&r, "https://notyoutube.com/"));
        assert!(rule_matches(&r, "https://reddit.com/r/all/top"));
        assert!(!rule_matches(&r, "https://reddit.com/r/rust"));
        assert!(!rule_matches(&r, "about:blank"));
    }

    #[test]
    fn old_rules_get_new_defaults() {
        let r: Rule = serde_json::from_str(r#"{"id":"a","name":"Old","sites":["x.com"],"mode":"pause","pause_seconds":12}"#).unwrap();
        assert_eq!(r.countdown, Countdown::Breathe);
        assert_eq!(r.prompts, vec![PromptKind::Reflect]);
        assert_eq!(r.pause_seconds, 12);
        let r: Rule = serde_json::from_str(r#"{"countdown":"drain","prompts":["cost","commit"]}"#).unwrap();
        assert_eq!(r.countdown, Countdown::Drain);
        assert_eq!(r.prompts, vec![PromptKind::Cost, PromptKind::Commit]);
    }

    #[test]
    fn old_settings_get_bedtime_off() {
        let c: Config = serde_json::from_str(r#"{"settings":{"sky":"dark"}}"#).unwrap();
        assert_eq!(c.settings.sky, "dark");
        assert!(!c.settings.bedtime);
        assert_eq!(c.settings.bedtime_at, "23:00");
    }

    #[test]
    fn schedules() {
        let mut s = Schedule { enabled: true, ..Schedule::default() }; // Mon–Fri 9–17
        // 2026-10-12 is a Monday
        assert!(schedule_active(&s, &at(2026, 10, 12, 9, 0)));
        assert!(!schedule_active(&s, &at(2026, 10, 12, 17, 0)));
        assert!(!schedule_active(&s, &at(2026, 10, 11, 12, 0))); // Sunday
        s.start = "22:00".into();
        s.end = "02:00".into();
        assert!(schedule_active(&s, &at(2026, 10, 12, 23, 0))); // Mon night
        assert!(schedule_active(&s, &at(2026, 10, 13, 1, 0))); // early Tue belongs to Mon
        assert!(!schedule_active(&s, &at(2026, 10, 12, 1, 0))); // early Mon belongs to Sun (off)
        s.enabled = false;
        assert!(schedule_active(&s, &at(2026, 10, 11, 1, 0)));
    }

    #[test]
    fn verdicts() {
        let now = at(2026, 10, 12, 10, 0);
        let mut cfg = Config::starter();
        cfg.rules[0].daily_opens = Some(2);
        cfg.rules[0].daily_minutes = Some(10);
        let mut stats = Stats::default();
        let no = |_: &str| false;
        let yes = |_: &str| true;
        let url = "https://youtube.com/";
        assert_eq!(evaluate(&cfg, &stats, &now, url, &no).unwrap().reason, Some(Reason::Pause));
        assert_eq!(evaluate(&cfg, &stats, &now, url, &yes).unwrap().reason, None);
        assert!(evaluate(&cfg, &stats, &now, "https://docs.rs/", &no).is_none());

        stats.day_mut("2026-10-12", "starter").opens = 2;
        assert_eq!(evaluate(&cfg, &stats, &now, url, &no).unwrap().reason, Some(Reason::OpenLimit));
        // an already-granted window still runs out normally
        assert_eq!(evaluate(&cfg, &stats, &now, url, &yes).unwrap().reason, None);

        stats.day_mut("2026-10-12", "starter").seconds = 600;
        assert_eq!(evaluate(&cfg, &stats, &now, url, &yes).unwrap().reason, Some(Reason::TimeLimit));

        cfg.rules.push(Rule {
            id: "work".into(),
            sites: vec!["youtube.com".into()],
            mode: Mode::Block,
            schedule: Schedule { enabled: true, ..Schedule::default() },
            ..Rule::default()
        });
        let v = evaluate(&cfg, &Stats::default(), &now, url, &yes).unwrap();
        assert_eq!((v.rule_index, v.reason), (1, Some(Reason::Blocked)));
        // outside work hours the block rule is dormant
        let v = evaluate(&cfg, &Stats::default(), &at(2026, 10, 12, 20, 0), url, &no).unwrap();
        assert_eq!((v.rule_index, v.reason), (0, Some(Reason::Pause)));
    }
}
