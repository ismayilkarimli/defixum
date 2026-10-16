//! Bedtime reminders: which reminder, if any, is due right now. Pure logic over a clock reading,
//! the chosen sleep time and what has already been shown, so the schedule can be tested without
//! a window or a real clock.

use chrono::{Duration, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Minutes before the sleep time at which a reminder appears. 0 is the sleep time itself; after it,
/// one every 15 minutes (negative) for two hours.
pub const OFFSETS: [i32; 14] = [120, 60, 30, 15, 5, 0, -15, -30, -45, -60, -75, -90, -105, -120];
/// How long after its moment a reminder can still appear (a Mac waking from sleep catches up).
const WINDOW: Duration = Duration::minutes(5);
/// A sleep time stays "the next one" this long after it passes, so the reminders after it have room.
const LINGER: Duration = Duration::minutes(130);
/// Reminder records kept on disk.
const KEEP: usize = 60;

/// What bedtime reminders have already happened. Saved so a restart neither repeats nor forgets.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Log {
    /// (night, offset) of every reminder shown or skipped
    pub shown: BTreeSet<(String, i32)>,
    /// the night whose remaining reminders were turned off with "Don't remind again today"
    pub muted: Option<String>,
}

/// A reminder that should appear now.
#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    /// date of the sleep time it belongs to, "YYYY-MM-DD"
    pub night: String,
    pub offset: i32,
    pub bedtime: NaiveDateTime,
}

pub fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

/// The sleep time these reminders are counting toward: the first one at or after 130 minutes ago.
pub fn next_bedtime(now: NaiveDateTime, at: NaiveTime) -> NaiveDateTime {
    let from = now - LINGER;
    let same_day = from.date().and_time(at);
    if same_day >= from { same_day } else { same_day + Duration::days(1) }
}

fn night_key(bedtime: NaiveDateTime) -> String {
    bedtime.format("%Y-%m-%d").to_string()
}

/// The reminder to show now, if any. Each is due for five minutes from its moment; if several are
/// (they aren't with the current offsets, but the rule is cheap), the nearest to bedtime wins.
pub fn due(now: NaiveDateTime, at: NaiveTime, log: &Log) -> Option<Due> {
    let bedtime = next_bedtime(now, at);
    let night = night_key(bedtime);
    if log.muted.as_deref() == Some(night.as_str()) {
        return None;
    }
    OFFSETS
        .iter()
        .rev()
        .find(|&&o| {
            let moment = bedtime - Duration::minutes(o as i64);
            now >= moment && now < moment + WINDOW && !log.shown.contains(&(night.clone(), o))
        })
        .map(|&offset| Due { night, offset, bedtime })
}

impl Log {
    /// Remember that this reminder was shown. Earlier ones for the same night count as done too.
    pub fn record(&mut self, night: &str, offset: i32) {
        for o in OFFSETS.into_iter().filter(|&o| o >= offset) {
            self.shown.insert((night.to_string(), o));
        }
        while self.shown.len() > KEEP {
            self.shown.pop_first();
        }
    }

    /// The sleep time was switched on or changed: start the coming night afresh, treating every
    /// reminder whose moment has already passed as done, so turning it on at 22:10 for 23:00
    /// doesn't pop the one-hour reminder at once.
    pub fn reschedule(&mut self, now: NaiveDateTime, at: NaiveTime) {
        let bedtime = next_bedtime(now, at);
        let night = night_key(bedtime);
        self.shown.retain(|(n, _)| *n != night);
        for o in OFFSETS {
            if bedtime - Duration::minutes(o as i64) <= now {
                self.record(&night, o);
            }
        }
    }

    /// "Don't remind again today": nothing more for this night.
    pub fn mute(&mut self, night: &str) {
        self.muted = Some(night.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// 11 October 2026 at a clock time; `t2` is the same time on the 12th.
    fn t(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 11).unwrap().and_time(at(h, m))
    }

    fn t2(h: u32, m: u32) -> NaiveDateTime {
        t(h, m) + Duration::days(1)
    }

    fn offset_at(now: NaiveDateTime, bed: NaiveTime, log: &Log) -> Option<i32> {
        due(now, bed, log).map(|d| d.offset)
    }

    #[test]
    fn parses_hours_and_minutes_only() {
        assert_eq!(parse_time("23:00"), Some(at(23, 0)));
        assert_eq!(parse_time(" 07:05 "), Some(at(7, 5)));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("late"), None);
        assert_eq!(parse_time(""), None);
    }

    #[test]
    fn quiet_through_a_normal_evening() {
        let log = Log::default();
        for (h, m) in [(18, 0), (20, 59), (21, 5), (21, 29), (22, 5), (22, 29), (22, 36), (22, 44), (22, 50), (22, 54)] {
            assert_eq!(offset_at(t(h, m), at(23, 0), &log), None, "{h}:{m:02}");
        }
    }

    #[test]
    fn each_offset_appears_at_its_moment() {
        let log = Log::default();
        let cases = [((21, 0), 120), ((22, 0), 60), ((22, 30), 30), ((22, 45), 15), ((22, 55), 5), ((23, 0), 0)];
        for ((h, m), want) in cases {
            let d = due(t(h, m), at(23, 0), &log).unwrap_or_else(|| panic!("nothing at {h}:{m:02}"));
            assert_eq!(d.offset, want, "{h}:{m:02}");
            assert_eq!(d.night, "2026-10-11");
            assert_eq!(d.bedtime, t(23, 0));
        }
    }

    #[test]
    fn a_reminder_lasts_five_minutes_and_not_longer() {
        let log = Log::default();
        assert_eq!(offset_at(t(22, 59), at(23, 0), &log), Some(5));
        assert_eq!(offset_at(t(22, 34), at(23, 0), &log), Some(30)); // woke late, still inside its window
        assert_eq!(offset_at(t(22, 35), at(23, 0), &log), None);
        assert_eq!(offset_at(t(22, 36), at(23, 0), &log), None);
    }

    #[test]
    fn the_bedtime_reminder_has_a_window_after_bedtime() {
        let log = Log::default();
        assert_eq!(offset_at(t(23, 4), at(23, 0), &log), Some(0));
        assert_eq!(offset_at(t(23, 5), at(23, 0), &log), None);
        assert_eq!(offset_at(t(23, 20), at(23, 0), &log), None);
    }

    #[test]
    fn every_quarter_hour_for_two_hours_after_bedtime() {
        let log = Log::default();
        let cases = [((23, 15), -15), ((23, 30), -30), ((23, 45), -45), ((0, 0), -60), ((0, 30), -90), ((1, 0), -120)];
        for ((h, m), want) in cases {
            let now = if h < 12 { t2(h, m) } else { t(h, m) };
            let d = due(now, at(23, 0), &log).unwrap_or_else(|| panic!("nothing at {h}:{m:02}"));
            assert_eq!((d.offset, d.night.as_str()), (want, "2026-10-11"), "{h}:{m:02}");
        }
        for (h, m) in [(23, 10), (0, 20), (1, 5), (1, 15), (2, 0), (3, 0)] {
            let now = if h < 12 { t2(h, m) } else { t(h, m) };
            assert_eq!(offset_at(now, at(23, 0), &log), None, "{h}:{m:02}");
        }
    }

    #[test]
    fn muting_after_bedtime_silences_the_rest() {
        let mut log = Log::default();
        let d = due(t(23, 30), at(23, 0), &log).unwrap();
        log.record(&d.night, d.offset);
        log.mute(&d.night);
        for (h, m) in [(23, 45), (0, 0), (0, 45), (1, 0)] {
            let now = if h < 12 { t2(h, m) } else { t(h, m) };
            assert_eq!(offset_at(now, at(23, 0), &log), None, "{h}:{m:02}");
        }
    }

    #[test]
    fn a_shown_reminder_is_not_shown_again() {
        let mut log = Log::default();
        assert_eq!(offset_at(t(22, 30), at(23, 0), &log), Some(30));
        log.record("2026-10-11", 30);
        assert_eq!(offset_at(t(22, 31), at(23, 0), &log), None);
        assert_eq!(offset_at(t(22, 45), at(23, 0), &log), Some(15));
    }

    #[test]
    fn showing_one_marks_the_earlier_ones_done() {
        let mut log = Log::default();
        log.record("2026-10-11", 30);
        for o in [120, 60, 30] {
            assert!(log.shown.contains(&("2026-10-11".to_string(), o)), "{o}");
        }
        assert!(!log.shown.contains(&("2026-10-11".to_string(), 15)));
    }

    #[test]
    fn muting_silences_the_rest_of_the_night() {
        let mut log = Log::default();
        log.mute("2026-10-11");
        for (h, m) in [(21, 0), (22, 0), (22, 30), (22, 45), (22, 55), (23, 0)] {
            assert_eq!(offset_at(t(h, m), at(23, 0), &log), None, "{h}:{m:02}");
        }
    }

    #[test]
    fn muting_does_not_reach_the_next_night() {
        let mut log = Log::default();
        log.mute("2026-10-11");
        assert_eq!(offset_at(t2(21, 0), at(23, 0), &log), Some(120));
        assert_eq!(offset_at(t2(22, 55), at(23, 0), &log), Some(5));
    }

    #[test]
    fn a_new_evening_starts_fresh() {
        let mut log = Log::default();
        for o in OFFSETS {
            log.record("2026-10-11", o);
        }
        assert_eq!(offset_at(t(23, 1), at(23, 0), &log), None);
        assert_eq!(offset_at(t2(21, 0), at(23, 0), &log), Some(120));
    }

    #[test]
    fn bedtime_after_midnight_counts_toward_the_next_date() {
        let log = Log::default();
        let d = due(t(22, 31), at(0, 30), &log).unwrap();
        assert_eq!(d.offset, 120);
        assert_eq!(d.night, "2026-10-12");
        assert_eq!(d.bedtime, t2(0, 30));

        assert_eq!(offset_at(t(23, 30), at(0, 30), &log), Some(60));
        assert_eq!(offset_at(t2(0, 0), at(0, 30), &log), Some(30));
        assert_eq!(offset_at(t2(0, 25), at(0, 30), &log), Some(5));
        let d = due(t2(0, 31), at(0, 30), &log).unwrap();
        assert_eq!((d.offset, d.night.as_str()), (0, "2026-10-12"));
        assert_eq!(offset_at(t2(0, 36), at(0, 30), &log), None);
    }

    #[test]
    fn mute_before_midnight_holds_after_midnight() {
        let mut log = Log::default();
        let first = due(t(22, 31), at(0, 30), &log).unwrap();
        log.mute(&first.night);
        assert_eq!(offset_at(t2(0, 31), at(0, 30), &log), None);
    }

    #[test]
    fn a_bedtime_at_midnight_sharp() {
        let log = Log::default();
        assert_eq!(offset_at(t(22, 0), at(0, 0), &log), Some(120));
        assert_eq!(offset_at(t2(0, 0), at(0, 0), &log), Some(0));
        assert_eq!(offset_at(t2(0, 2), at(0, 0), &log), Some(0));
    }

    #[test]
    fn the_next_bedtime_is_today_or_tomorrow() {
        assert_eq!(next_bedtime(t(10, 0), at(23, 0)), t(23, 0));
        assert_eq!(next_bedtime(t(23, 10), at(23, 0)), t(23, 0)); // within the half hour after
        assert_eq!(next_bedtime(t(23, 30), at(23, 0)), t(23, 0));
        assert_eq!(next_bedtime(t2(1, 10), at(23, 0)), t(23, 0)); // the last reminder's window
        assert_eq!(next_bedtime(t2(1, 11), at(23, 0)), t2(23, 0));
        assert_eq!(next_bedtime(t(0, 10), at(23, 0)), t(23, 0) - Duration::days(1)); // last night's
        assert_eq!(next_bedtime(t(1, 30), at(23, 0)), t(23, 0));
    }

    #[test]
    fn switching_it_on_mid_evening_skips_what_has_passed() {
        let mut log = Log::default();
        log.reschedule(t(22, 10), at(23, 0));
        // 22:10 for 23:00: the 2 h and 1 h moments are behind us
        assert_eq!(offset_at(t(22, 10), at(23, 0), &log), None);
        assert_eq!(offset_at(t(22, 31), at(23, 0), &log), Some(30));
        assert_eq!(offset_at(t(22, 46), at(23, 0), &log), Some(15));
    }

    #[test]
    fn switching_it_on_just_after_a_moment_does_not_pop_it() {
        let mut log = Log::default();
        log.reschedule(t(22, 2), at(23, 0));
        assert_eq!(offset_at(t(22, 2), at(23, 0), &log), None); // the 1 h window is still open
        assert_eq!(offset_at(t(22, 31), at(23, 0), &log), Some(30));
    }

    #[test]
    fn switching_it_on_after_bedtime_waits_for_the_next_quarter_hour() {
        let mut log = Log::default();
        log.reschedule(t(23, 2), at(23, 0));
        assert_eq!(offset_at(t(23, 3), at(23, 0), &log), None);
        assert_eq!(offset_at(t(23, 15), at(23, 0), &log), Some(-15));
        assert_eq!(offset_at(t2(21, 0), at(23, 0), &log), Some(120));
    }

    #[test]
    fn switching_it_on_early_leaves_the_whole_evening() {
        let mut log = Log::default();
        log.reschedule(t(15, 0), at(23, 0));
        assert!(log.shown.is_empty());
        assert_eq!(offset_at(t(21, 0), at(23, 0), &log), Some(120));
    }

    #[test]
    fn changing_the_time_forgets_what_the_old_time_showed() {
        let mut log = Log::default();
        // the 22:00 bedtime showed its 2 h reminder at 20:00
        log.record("2026-10-11", 120);
        // at 20:10 it moves to 23:30; the 21:30 reminder should still come
        log.reschedule(t(20, 10), at(23, 30));
        assert_eq!(offset_at(t(21, 30), at(23, 30), &log), Some(120));
    }

    #[test]
    fn rescheduling_keeps_a_mute() {
        let mut log = Log::default();
        log.mute("2026-10-11");
        log.reschedule(t(20, 0), at(23, 0));
        assert_eq!(offset_at(t(22, 30), at(23, 0), &log), None);
    }

    #[test]
    fn the_log_stays_small() {
        let mut log = Log::default();
        for d in 1..=20 {
            log.record(&format!("2026-09-{d:02}"), 0);
        }
        for d in 1..=20 {
            log.record(&format!("2026-10-{d:02}"), 120);
        }
        assert!(log.shown.len() <= KEEP);
        assert!(log.shown.contains(&("2026-10-20".to_string(), 120)));
        assert!(!log.shown.contains(&("2026-09-01".to_string(), 0)));
    }

    #[test]
    fn the_log_survives_a_round_trip() {
        let mut log = Log::default();
        log.record("2026-10-11", 15);
        log.mute("2026-10-11");
        let text = serde_json::to_string(&log).unwrap();
        assert_eq!(serde_json::from_str::<Log>(&text).unwrap(), log);
        assert_eq!(serde_json::from_str::<Log>("{}").unwrap(), Log::default());
    }
}
