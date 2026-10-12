//! Thin wrappers over the macOS APIs Defixum needs: the Accessibility API, the frontmost
//! app, idle time, synthetic keys, and in-process AppleScript.

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::CFURL;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::CFTypeID;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::null;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementGetTypeID() -> CFTypeID;
    fn AXUIElementCopyAttributeValue(el: CFTypeRef, attr: CFStringRef, value: *mut CFTypeRef) -> i32;
    fn AXUIElementSetAttributeValue(el: CFTypeRef, attr: CFStringRef, value: CFTypeRef) -> i32;
    fn AXUIElementPerformAction(el: CFTypeRef, action: CFStringRef) -> i32;
    fn AXUIElementSetMessagingTimeout(el: CFTypeRef, seconds: f32) -> i32;
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> CFTypeRef;
    fn TISCopyCurrentASCIICapableKeyboardLayoutInputSource() -> CFTypeRef;
    fn TISGetInputSourceProperty(source: CFTypeRef, key: CFStringRef) -> CFTypeRef;
    fn UCKeyTranslate(
        layout: *const u8,
        key_code: u16,
        action: u16,
        modifiers: u32,
        keyboard_type: u32,
        options: u32,
        dead_keys: *mut u32,
        max_len: usize,
        actual_len: *mut usize,
        out: *mut u16,
    ) -> i32;
    fn LMGetKbdType() -> u8;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}

// ---------- Accessibility ----------

pub fn ax_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Shows the system "allow accessibility" prompt if not yet trusted.
pub fn ax_prompt() -> bool {
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as *const c_void) != 0 }
}

#[derive(Clone)]
pub struct Element(CFType);

impl Element {
    pub fn application(pid: i32) -> Option<Element> {
        let raw = unsafe { AXUIElementCreateApplication(pid) };
        if raw.is_null() {
            return None;
        }
        let el = Element(unsafe { CFType::wrap_under_create_rule(raw) });
        unsafe { AXUIElementSetMessagingTimeout(el.raw(), 0.5) };
        Some(el)
    }

    fn raw(&self) -> CFTypeRef {
        self.0.as_CFTypeRef()
    }

    pub fn attr(&self, name: &str) -> Option<CFType> {
        let key = CFString::new(name);
        let mut value: CFTypeRef = null();
        let err = unsafe { AXUIElementCopyAttributeValue(self.raw(), key.as_concrete_TypeRef(), &mut value) };
        if err != 0 || value.is_null() {
            return None;
        }
        Some(unsafe { CFType::wrap_under_create_rule(value) })
    }

    pub fn string(&self, name: &str) -> Option<String> {
        self.attr(name)?.downcast::<CFString>().map(|s| s.to_string())
    }

    /// A numeric attribute (e.g. `AXMenuItemCmdModifiers`).
    pub fn int(&self, name: &str) -> Option<i64> {
        self.attr(name)?.downcast::<CFNumber>()?.to_i64()
    }

    /// A boolean attribute (e.g. `AXEnabled`).
    pub fn flag(&self, name: &str) -> Option<bool> {
        self.attr(name)?.downcast::<CFBoolean>().map(bool::from)
    }

    pub fn role(&self) -> String {
        self.string("AXRole").unwrap_or_default()
    }

    pub fn element(&self, name: &str) -> Option<Element> {
        let v = self.attr(name)?;
        (v.type_of() == unsafe { AXUIElementGetTypeID() }).then(|| Element(v))
    }

    pub fn url(&self) -> Option<String> {
        let v = self.attr("AXURL")?;
        if let Some(u) = v.downcast::<CFURL>() {
            return Some(u.get_string().to_string());
        }
        v.downcast::<CFString>().map(|s| s.to_string())
    }

    pub fn children(&self) -> Vec<Element> {
        let Some(v) = self.attr("AXChildren") else { return vec![] };
        if v.type_of() != unsafe { CFArrayGetTypeID() } {
            return vec![];
        }
        let arr = v.as_CFTypeRef() as CFArrayRef;
        let n = unsafe { CFArrayGetCount(arr) };
        (0..n)
            .filter_map(|i| {
                let p = unsafe { CFArrayGetValueAtIndex(arr, i) };
                (!p.is_null()).then(|| Element(unsafe { CFType::wrap_under_get_rule(p) }))
            })
            .collect()
    }

    pub fn set_bool(&self, name: &str, value: bool) -> i32 {
        let key = CFString::new(name);
        let v = if value { CFBoolean::true_value() } else { CFBoolean::false_value() };
        unsafe { AXUIElementSetAttributeValue(self.raw(), key.as_concrete_TypeRef(), v.as_CFTypeRef()) }
    }

    pub fn set_string(&self, name: &str, value: &str) -> i32 {
        let key = CFString::new(name);
        let v = CFString::new(value);
        unsafe { AXUIElementSetAttributeValue(self.raw(), key.as_concrete_TypeRef(), v.as_CFTypeRef()) }
    }

    /// Performs an accessibility action such as `AXPress`; 0 means it was carried out.
    pub fn press(&self) -> i32 {
        let action = CFString::new("AXPress");
        unsafe { AXUIElementPerformAction(self.raw(), action.as_concrete_TypeRef()) }
    }

    /// Breadth-first search below this element. `descend` decides whether to look inside a
    /// node's children; `limit` caps the number of nodes visited (each costs an IPC round trip).
    pub fn find(
        &self,
        limit: usize,
        hit: &dyn Fn(&Element, &str) -> bool,
        descend: &dyn Fn(&str) -> bool,
    ) -> Option<Element> {
        let mut queue = std::collections::VecDeque::from([self.clone()]);
        let mut seen = 0;
        while let Some(e) = queue.pop_front() {
            seen += 1;
            if seen > limit {
                return None;
            }
            let role = e.role();
            if hit(&e, &role) {
                return Some(e);
            }
            if descend(&role) {
                queue.extend(e.children());
            }
        }
        None
    }
}

// ---------- apps ----------

pub struct FrontApp {
    pub bundle: String,
    pub pid: i32,
}

pub fn frontmost() -> Option<FrontApp> {
    use objc2_app_kit::NSWorkspace;
    objc2::rc::autoreleasepool(|_| {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let bundle = app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();
        Some(FrontApp { bundle, pid: app.processIdentifier() })
    })
}

pub fn activate(pid: i32) {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    objc2::rc::autoreleasepool(|_| {
        if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        }
    });
}

pub fn is_running(bundle: &str) -> bool {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    objc2::rc::autoreleasepool(|_| {
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle)).count() > 0
    })
}

pub fn pid_of(bundle: &str) -> Option<i32> {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    objc2::rc::autoreleasepool(|_| {
        let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle));
        apps.firstObject().map(|a| a.processIdentifier())
    })
}

pub fn app_installed(bundle: &str) -> bool {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    objc2::rc::autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle)).is_some()
    })
}

/// Seconds since the last keyboard/mouse/trackpad input anywhere.
pub fn idle_seconds() -> f64 {
    // kCGEventSourceStateCombinedSessionState = 0, kCGAnyInputEventType = ~0
    unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) }
}

/// Sends a key press straight to one process.
pub fn press_key(pid: i32, keycode: u16) {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
    for down in [true, false] {
        if let Ok(ev) = CGEvent::new_keyboard_event(source.clone(), keycode, down) {
            ev.post_to_pid(pid);
        }
    }
}

/// The character a virtual key types in the current keyboard layout (key 13 is W: "Ü" on an
/// Azeri layout). Menu shortcuts are reported in that form. The TIS calls assert off the main
/// thread on macOS 14+, so call this from the main thread.
pub fn key_char(keycode: u16) -> Option<String> {
    use core_foundation_sys::base::CFRelease;
    use core_foundation_sys::data::CFDataGetBytePtr;
    unsafe fn translate(source: CFTypeRef, keycode: u16) -> Option<String> {
        if source.is_null() {
            return None;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        let out = if data.is_null() {
            None
        } else {
            // kUCKeyActionDisplay = 3, kUCKeyTranslateNoDeadKeysMask = 1
            let (mut dead, mut len, mut buf) = (0u32, 0usize, [0u16; 4]);
            let layout = CFDataGetBytePtr(data as _);
            let status = UCKeyTranslate(layout, keycode, 3, 0, LMGetKbdType() as u32, 1, &mut dead, 4, &mut len, buf.as_mut_ptr());
            (status == 0 && len > 0).then(|| String::from_utf16_lossy(&buf[..len.min(4)]))
        };
        CFRelease(source);
        out
    }
    unsafe {
        // IME sources carry no layout data; their ASCII-capable layout stands in
        translate(TISCopyCurrentKeyboardLayoutInputSource(), keycode)
            .or_else(|| translate(TISCopyCurrentASCIICapableKeyboardLayoutInputSource(), keycode))
    }
}

/// Whether a menu item's shortcut character is `key`, ignoring case ("ü" equals "Ü").
pub fn same_key(cmd_char: &str, key: &str) -> bool {
    !key.is_empty() && cmd_char.to_lowercase() == key.to_lowercase()
}

// ---------- AppleScript ----------

#[derive(Debug, Clone, PartialEq)]
pub enum Script {
    Ok(String),
    /// The user hasn't allowed (or has denied) Defixum controlling that app.
    Denied,
    Failed(String),
}

thread_local! {
    static SCRIPTS: RefCell<HashMap<String, objc2::rc::Retained<objc2_foundation::NSAppleScript>>> =
        RefCell::new(HashMap::new());
}

/// Runs AppleScript in-process. Must be called on the main thread. Compiled scripts are
/// cached, so repeated polls cost a single Apple Event round trip.
pub fn applescript(source: &str) -> Script {
    use objc2::AnyThread;
    use objc2_foundation::{NSAppleScript, NSNumber, NSString};
    objc2::rc::autoreleasepool(|_| {
        let script = SCRIPTS.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(s) = cache.get(source) {
                return Some(s.clone());
            }
            let s = NSAppleScript::initWithSource(NSAppleScript::alloc(), &NSString::from_str(source))?;
            cache.insert(source.to_string(), s.clone());
            Some(s)
        });
        let Some(script) = script else { return Script::Failed("could not create script".into()) };
        let mut error: Option<objc2::rc::Retained<objc2_foundation::NSDictionary<NSString, objc2::runtime::AnyObject>>> = None;
        // Not `script.executeAndReturnError`: objc2 types its result as non-null and panics when
        // a script fails (e.g. times out while macOS shows the Automation prompt), killing the app.
        let result: Option<objc2::rc::Retained<objc2_foundation::NSAppleEventDescriptor>> =
            unsafe { objc2::msg_send![&*script, executeAndReturnError: Some(&mut error)] };
        if let Some(err) = error {
            let number = err
                .objectForKey(&NSString::from_str("NSAppleScriptErrorNumber"))
                .and_then(|n| n.downcast::<NSNumber>().ok())
                .map(|n| n.intValue())
                .unwrap_or(0);
            let message = err
                .objectForKey(&NSString::from_str("NSAppleScriptErrorMessage"))
                .and_then(|n| n.downcast::<NSString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default();
            // -1743: not authorized to send Apple events; -1744: would need to ask
            return if number == -1743 || number == -1744 {
                Script::Denied
            } else {
                Script::Failed(format!("{number}: {message}"))
            };
        }
        match result {
            Some(r) => Script::Ok(r.stringValue().map(|s| s.to_string()).unwrap_or_default()),
            None => Script::Failed("script returned nothing".into()),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_characters_compare_ignoring_case() {
        assert!(same_key("W", "w"));
        assert!(same_key("Ü", "ü"));
        assert!(!same_key("W", ""));
        assert!(!same_key("Q", "w"));
    }

    #[test]
    fn failing_script_is_an_error_not_a_crash() {
        match applescript("error \"boom\" number 42") {
            Script::Failed(msg) => assert_eq!(msg, "42: boom"),
            _ => panic!("expected Script::Failed"),
        }
        assert!(matches!(applescript("return \"ok\""), Script::Ok(s) if s == "ok"));
    }
}
