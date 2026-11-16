//! The app's data as the windows see it. Fields the windows don't touch ride along in `rest`, so
//! saving the settings never drops what a newer app stores.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub rules: Vec<Rule>,
    pub settings: Settings,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub sites: Vec<String>,
    /// "pause" or "block"
    pub mode: String,
    pub pause_seconds: u32,
    /// "breathe", "fill" or "drain"
    pub countdown: String,
    /// "reflect", "cost", "commit"
    pub prompts: Vec<String>,
    pub daily_opens: Option<u32>,
    pub daily_minutes: Option<u32>,
    pub separate_limits: bool,
    pub schedule: Schedule,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            id: String::new(),
            name: "New rule".into(),
            enabled: true,
            sites: vec![],
            mode: "pause".into(),
            pause_seconds: 10,
            countdown: "breathe".into(),
            prompts: vec!["reflect".into()],
            daily_opens: None,
            daily_minutes: None,
            separate_limits: false,
            schedule: Schedule::default(),
            rest: Map::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub enabled: bool,
    pub days: [bool; 7],
    pub start: String,
    pub end: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule { enabled: false, days: [true, true, true, true, true, false, false], start: "00:00".into(), end: "23:59".into() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub session_options: Vec<u32>,
    pub idle_seconds: u64,
    pub sky: String,
    pub weather: bool,
    pub bedtime: bool,
    pub bedtime_at: String,
    pub bedtime_by_day: Vec<String>,
    pub switch_limit: bool,
    pub switches_per_minute: u32,
    pub switch_cooldown_seconds: u32,
    pub move_reminder: bool,
    pub move_every_minutes: u32,
    pub move_breaks: Vec<u32>,
    pub move_prompt_seconds: u32,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
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
            bedtime_by_day: vec![String::new(); 7],
            switch_limit: false,
            switches_per_minute: 6,
            switch_cooldown_seconds: 60,
            move_reminder: true,
            move_every_minutes: 45,
            move_breaks: vec![1, 3, 5, 10],
            move_prompt_seconds: 10,
            rest: Map::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DayStats {
    pub intercepts: u32,
    pub opens: u32,
    pub skips: u32,
    pub seconds: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BrowserInfo {
    pub bundle: String,
    pub name: String,
    /// "ax", "chromium" or "safari"
    pub kind: String,
    pub running: bool,
    /// "ok", "denied" or none yet
    pub status: Option<String>,
}

/// What the settings window shows (`get_state`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct DashState {
    pub config: Config,
    pub today: String,
    pub stats: BTreeMap<String, HashMap<String, DayStats>>,
    pub accessibility: bool,
    pub browsers: Vec<BrowserInfo>,
    pub autostart: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Prompt {
    pub kind: String,
    pub text: String,
    pub sentence: Option<String>,
}

/// A pause or block on the overlay (`intercept`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Intercept {
    pub rule_id: String,
    pub rule_name: String,
    pub site: String,
    /// "pause", "expired", "blocked", "open_limit" or "time_limit"
    pub reason: String,
    pub pause_seconds: u32,
    pub countdown: String,
    pub prompt: Option<Prompt>,
    pub opens_today: u32,
    pub minutes_today: u64,
    pub daily_opens: Option<u32>,
    pub daily_minutes: Option<u32>,
    pub session_options: Vec<u32>,
    pub until: Option<String>,
    pub sky: String,
    pub weather: Option<crate::sky::Reading>,
    pub waited: bool,
}

/// A bedtime reminder (`bedtime`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Bedtime {
    pub offset: i32,
    pub minutes_left: i64,
    pub bedtime: String,
    pub bedtime_ts: i64,
    pub snooze: Vec<u32>,
    pub sky: String,
    pub weather: Option<crate::sky::Reading>,
}

/// The reminder to get up (`move`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Move {
    /// "prompt" or "break"
    pub phase: String,
    /// minutes at the screen when it came up
    pub sitting: u32,
    pub breaks: Vec<u32>,
    /// when this phase ends by itself, unix milliseconds
    pub ends_at: f64,
    /// the phase's whole length, seconds
    pub seconds: u32,
    pub sky: String,
    pub weather: Option<crate::sky::Reading>,
}

/// The toast (`toast`): ⌘Tab resting, or a note to get up.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Toast {
    /// "switch" or "move"
    pub kind: String,
    pub minutes: u32,
    pub seconds: u32,
    pub holding: bool,
    pub hold_ms: u32,
    /// end of the cooldown, unix milliseconds
    pub until: f64,
    pub limit: u32,
}
