//! Limits Cmd+Tab. An event tap counts app switches (one per Cmd+Tab press, however many times
//! Tab is tapped while Cmd stays down) and, once more than the allowed number land within a
//! minute, swallows Cmd+Tab for a cooldown so the app switcher doesn't open. Holding Cmd+Tab
//! down for `HOLD_THROUGH` lifts the cooldown and opens the switcher anyway. Needs the same
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
/// How long Cmd+Tab has to be held down to get past the cooldown.
pub const HOLD_THROUGH: Duration = Duration::from_secs(5);

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

    /// The user held on through the cooldown: it ends, and the count starts fresh.
    pub fn lift(&mut self) {
        self.until = None;
        self.recent.clear();
    }
}

/// One Cmd press.
#[derive(Default)]
struct Hold {
    /// a Cmd+Tab in this press has been counted
    counted: bool,
    /// and refused: its Tabs are swallowed
    blocked: bool,
    /// when the refused Tab went down, while it is still down
    tab_since: Option<Instant>,
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static LIMIT: AtomicU32 = AtomicU32::new(6);
static COOLDOWN_SECS: AtomicU32 = AtomicU32::new(60);
static STARTED: AtomicBool = AtomicBool::new(false);
static PORT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static LIMITER: Mutex<Option<Limiter>> = Mutex::new(None);
/// Told when Cmd+Tab was swallowed, with the end of the cooldown and the limit.
static ON_BLOCK: OnceLock<Box<dyn Fn(Instant, u32) + Send + Sync>> = OnceLock::new();
/// Told when a long hold got past the cooldown.
static ON_LIFT: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

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

/// Called once at launch with what to do when Cmd+Tab is swallowed, and when a long hold gets
/// past the cooldown.
pub fn init(on_block: impl Fn(Instant, u32) + Send + Sync + 'static, on_lift: impl Fn() + Send + Sync + 'static) {
    let _ = ON_BLOCK.set(Box::new(on_block));
    let _ = ON_LIFT.set(Box::new(on_lift));
}

/// Ends the cooldown after a long hold and tells the app.
fn lift() {
    if let Some(l) = LIMITER.lock().unwrap().as_mut() {
        l.lift();
    }
    crate::log("cmd+tab limit: held through the cooldown");
    if let Some(tell) = ON_LIFT.get() {
        tell();
    }
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
        let hold: Mutex<Hold> = Mutex::new(Hold::default());
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
                            *hold = Hold::default();
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
                let down = matches!(kind, CGEventType::KeyDown);
                let repeat = event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0;
                if hold.blocked {
                    let held = hold.tab_since.is_some_and(|t| t.elapsed() >= HOLD_THROUGH);
                    if down && !repeat {
                        // Tab let go and pressed again under the same Cmd: a new hold
                        hold.tab_since = Some(Instant::now());
                    } else if down && held {
                        // Tab is still down (the keyboard repeats it): let this press through as a
                        // fresh one, so the switcher opens now
                        hold.blocked = false;
                        hold.tab_since = None;
                        lift();
                        event.set_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT, 0);
                        return CallbackResult::Keep;
                    } else if !down {
                        // key repeat turned off: no repeats arrive, so the next press goes through
                        if held {
                            hold.blocked = false;
                            lift();
                        }
                        hold.tab_since = None;
                    }
                }
                if down && !hold.counted {
                    let limit = LIMIT.load(Ordering::SeqCst);
                    let cooldown = Duration::from_secs(COOLDOWN_SECS.load(Ordering::SeqCst) as u64);
                    let verdict = LIMITER
                        .lock()
                        .unwrap()
                        .get_or_insert_with(Limiter::default)
                        .attempt(Instant::now(), limit, cooldown);
                    hold.counted = true;
                    hold.blocked = verdict != Verdict::Allow;
                    hold.tab_since = hold.blocked.then(Instant::now);
                    if let (Verdict::Block(until), Some(tell)) = (verdict, ON_BLOCK.get()) {
                        tell(until, limit);
                    }
                }
                if hold.blocked {
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
    fn holding_through_lifts_the_cooldown() {
        let mut l = Limiter::default();
        let t = Instant::now();
        for i in 0..3 {
            l.attempt(t + Duration::from_secs(i), 3, COOLDOWN);
        }
        assert!(matches!(l.attempt(t + Duration::from_secs(4), 3, COOLDOWN), Verdict::Block(_)));
        l.lift();
        // a fresh minute: the whole allowance again
        for i in 0..3 {
            assert_eq!(l.attempt(t + Duration::from_secs(10 + i), 3, COOLDOWN), Verdict::Allow);
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
