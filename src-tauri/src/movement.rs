//! The reminder to get up counts time while the display is awake, including time without input;
//! the display going to sleep is a break and starts the count over.

use std::time::Duration;

#[derive(Default)]
pub struct Clock {
    sitting: Duration,
}

impl Clock {
    /// Adds `dt` of screen time and returns whether the reminder is due.
    pub fn tick(&mut self, dt: Duration, every: Duration) -> bool {
        self.sitting += dt;
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

    const EVERY: Duration = Duration::from_secs(45 * 60);

    #[test]
    fn due_after_the_configured_screen_time() {
        let mut c = Clock::default();
        let second = Duration::from_secs(1);
        for _ in 0..(EVERY.as_secs() - 1) {
            assert!(!c.tick(second, EVERY));
        }
        assert!(c.tick(second, EVERY));
    }

    #[test]
    fn reset_restarts_the_interval() {
        let mut c = Clock::default();
        assert!(c.tick(EVERY, EVERY));
        c.reset();
        assert_eq!(c.sitting(), Duration::ZERO);
        assert!(!c.tick(Duration::from_secs(1), EVERY));
    }

    #[test]
    fn breaks_are_cleaned() {
        assert_eq!(clean_breaks(&[10, 0, 3, 3, 90, 1, 5, 7]), vec![1, 3, 5, 7]);
    }
}
