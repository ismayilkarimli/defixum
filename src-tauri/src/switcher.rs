//! Limits Cmd+Tab. An event tap counts app switches (one per Cmd+Tab press, however many times
//! Tab is tapped while Cmd stays down) and, once more than the allowed number land within a
//! minute, swallows Cmd+Tab for a cooldown so the app switcher doesn't open. Needs the same
//! Accessibility permission the browser reading uses.

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const TAB: i64 = 48;

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Allow,
    /// Cmd+Tab is off until this instant.
    Block(Instant),
}

/// Switches in the last minute, and the cooldown when there is one.
#[derive(Default)]
pub struct Limiter {
    recent: VecDeque<Instant>,
    until: Option<Instant>,
}

impl Limiter {
    /// A Cmd+Tab press at `now`, with `limit` switches allowed per minute and Cmd+Tab off for
    /// `cooldown` once that is passed.
    pub fn attempt(&mut self, now: Instant, limit: u32, cooldown: Duration) -> Verdict {
        if let Some(until) = self.until {
            if now < until {
                return Verdict::Block(until);
            }
            self.until = None;
        }
        while self.recent.front().is_some_and(|&t| now.duration_since(t) >= WINDOW) {
            self.recent.pop_front();
        }
        if self.recent.len() >= limit as usize {
            // a clean slate after the cooldown
            self.recent.clear();
            let until = now + cooldown;
            self.until = Some(until);
            return Verdict::Block(until);
        }
        self.recent.push_back(now);
        Verdict::Allow
    }
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static LIMIT: AtomicU32 = AtomicU32::new(6);
static COOLDOWN_SECS: AtomicU32 = AtomicU32::new(60);
static STARTED: AtomicBool = AtomicBool::new(false);
static PORT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static LIMITER: Mutex<Option<Limiter>> = Mutex::new(None);
/// Told when Cmd+Tab was swallowed, with the end of the cooldown and the limit.
static ON_BLOCK: OnceLock<Box<dyn Fn(Instant, u32) + Send + Sync>> = OnceLock::new();

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

fn set_tap(on: bool) {
    let port = PORT.load(Ordering::SeqCst);
    if !port.is_null() {
        unsafe { CGEventTapEnable(port, on) };
    }
}

/// Called once at launch with what to do when Cmd+Tab is swallowed.
pub fn init(on_block: impl Fn(Instant, u32) + Send + Sync + 'static) {
    let _ = ON_BLOCK.set(Box::new(on_block));
}

/// Applies the settings. Cheap; call it whenever they change and now and then, so the tap
/// starts once Accessibility is granted.
pub fn sync(enabled: bool, limit: u32, cooldown_secs: u32) {
    LIMIT.store(limit.max(1), Ordering::SeqCst);
    COOLDOWN_SECS.store(cooldown_secs.max(1), Ordering::SeqCst);
    let was = ENABLED.swap(enabled, Ordering::SeqCst);
    if was && !enabled {
        *LIMITER.lock().unwrap() = None;
    }
    if enabled && !STARTED.load(Ordering::SeqCst) && crate::macos::ax_trusted() {
        start();
    }
    set_tap(enabled);
}

fn start() {
    STARTED.store(true, Ordering::SeqCst);
    std::thread::spawn(|| {
        // whether this Cmd press already counted as a switch, and if that switch was refused
        let hold: Mutex<(bool, bool)> = Mutex::new((false, false));
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
            move |_, kind, event| {
                let mut hold = hold.lock().unwrap();
                match kind {
                    // macOS turns a slow tap off; turn it back on
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                        set_tap(ENABLED.load(Ordering::SeqCst));
                        return CallbackResult::Keep;
                    }
                    CGEventType::FlagsChanged => {
                        if !event.get_flags().contains(CGEventFlags::CGEventFlagCommand) {
                            *hold = (false, false);
                        }
                        return CallbackResult::Keep;
                    }
                    _ => {}
                }
                let cmd = event.get_flags().contains(CGEventFlags::CGEventFlagCommand);
                if !ENABLED.load(Ordering::SeqCst)
                    || !cmd
                    || event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) != TAB
                {
                    return CallbackResult::Keep;
                }
                if matches!(kind, CGEventType::KeyDown) && !hold.0 {
                    let limit = LIMIT.load(Ordering::SeqCst);
                    let cooldown = Duration::from_secs(COOLDOWN_SECS.load(Ordering::SeqCst) as u64);
                    let verdict = LIMITER
                        .lock()
                        .unwrap()
                        .get_or_insert_with(Limiter::default)
                        .attempt(Instant::now(), limit, cooldown);
                    *hold = (true, verdict != Verdict::Allow);
                    if let (Verdict::Block(until), Some(tell)) = (verdict, ON_BLOCK.get()) {
                        tell(until, limit);
                    }
                }
                if hold.1 {
                    CallbackResult::Drop
                } else {
                    CallbackResult::Keep
                }
            },
        );
        let Ok(tap) = tap else {
            crate::log("cmd+tab limit: could not install the key tap");
            STARTED.store(false, Ordering::SeqCst);
            return;
        };
        let Ok(source) = tap.mach_port().create_runloop_source(0) else {
            STARTED.store(false, Ordering::SeqCst);
            return;
        };
        PORT.store(tap.mach_port().as_concrete_TypeRef() as *mut c_void, Ordering::SeqCst);
        CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
        set_tap(ENABLED.load(Ordering::SeqCst));
        crate::log("cmd+tab limit: key tap installed");
        CFRunLoop::run_current();
        drop(tap);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const COOLDOWN: Duration = Duration::from_secs(60);

    #[test]
    fn allows_the_limit_then_cools_down() {
        let mut l = Limiter::default();
        let t = Instant::now();
        for i in 0..6 {
            assert_eq!(l.attempt(t + Duration::from_secs(i), 6, COOLDOWN), Verdict::Allow);
        }
        let seventh = t + Duration::from_secs(10);
        assert_eq!(l.attempt(seventh, 6, COOLDOWN), Verdict::Block(seventh + COOLDOWN));
        // still off near the end of the cooldown, on again after it
        assert_eq!(l.attempt(seventh + COOLDOWN - Duration::from_secs(1), 6, COOLDOWN), Verdict::Block(seventh + COOLDOWN));
        for i in 0..6 {
            assert_eq!(l.attempt(seventh + COOLDOWN + Duration::from_secs(i), 6, COOLDOWN), Verdict::Allow);
        }
    }

    #[test]
    fn only_the_last_minute_counts() {
        let mut l = Limiter::default();
        let t = Instant::now();
        // two a minute forever stays under a limit of three
        for i in 0..20 {
            assert_eq!(l.attempt(t + Duration::from_secs(i * 30), 3, COOLDOWN), Verdict::Allow);
        }
        let t = t + Duration::from_secs(20 * 30 + 60);
        for i in 0..3 {
            assert_eq!(l.attempt(t + Duration::from_secs(i), 3, COOLDOWN), Verdict::Allow);
        }
        assert!(matches!(l.attempt(t + Duration::from_secs(5), 3, COOLDOWN), Verdict::Block(_)));
    }
}
