//! The reminder to get up: how long the user has been at the screen without a break.
//!
//! Time counts while the screen is on. Being away from the keyboard and mouse for `AWAY` is a
//! break and starts the count over, unless something is holding the screen awake (a video, a
//! call): sitting still in front of a film is still sitting.

use std::time::Duration;

/// No input for this long, with nothing playing, means the user stepped away.
pub const AWAY: Duration = Duration::from_secs(3 * 60);

#[derive(Default)]
pub struct Clock {
    sitting: Duration,
}

impl Clock {
    /// Adds `dt` of screen time. `idle` is the time since the last input; `busy` is whether a
    /// video or call is on. Returns whether the reminder is due.
    pub fn tick(&mut self, dt: Duration, idle: Duration, busy: bool, every: Duration) -> bool {
        if idle >= AWAY && !busy {
            self.sitting = Duration::ZERO;
        } else {
            self.sitting += dt;
        }
        self.sitting >= every
    }

    pub fn sitting(&self) -> Duration {
        self.sitting
    }

    /// The reminder was shown, or a break taken: the next one is a full interval away.
    pub fn reset(&mut self) {
        self.sitting = Duration::ZERO;
    }
}

/// The break choices as offered: whole minutes from 1 to 60, sorted, at most four.
pub fn clean_breaks(list: &[u32]) -> Vec<u32> {
    let mut v: Vec<u32> = list.iter().copied().filter(|m| (1..=60).contains(m)).collect();
    v.sort_unstable();
    v.dedup();
    v.truncate(4);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);
    const EVERY: Duration = Duration::from_secs(45 * 60);

    fn run(c: &mut Clock, secs: u64, idle: Duration, busy: bool) -> bool {
        let mut due = false;
        for _ in 0..secs {
            due = c.tick(S, idle, busy, EVERY);
        }
        due
    }

    #[test]
    fn due_after_the_interval_of_use() {
        let mut c = Clock::default();
        assert!(!run(&mut c, 45 * 60 - 1, Duration::ZERO, false));
        assert!(run(&mut c, 1, Duration::ZERO, false));
    }

    #[test]
    fn stepping_away_starts_over() {
        let mut c = Clock::default();
        run(&mut c, 40 * 60, Duration::ZERO, false);
        run(&mut c, 1, AWAY, false);
        assert_eq!(c.sitting(), Duration::ZERO);
    }

    #[test]
    fn a_short_pause_is_not_a_break() {
        let mut c = Clock::default();
        run(&mut c, 40 * 60, Duration::ZERO, false);
        run(&mut c, 60, AWAY - S, false);
        assert_eq!(c.sitting(), Duration::from_secs(41 * 60));
    }

    #[test]
    fn watching_a_video_still_counts() {
        let mut c = Clock::default();
        assert!(run(&mut c, 45 * 60, Duration::from_secs(20 * 60), true));
    }

    #[test]
    fn breaks_are_cleaned() {
        assert_eq!(clean_breaks(&[10, 0, 3, 3, 90, 1, 5, 7]), vec![1, 3, 5, 7]);
    }
}
