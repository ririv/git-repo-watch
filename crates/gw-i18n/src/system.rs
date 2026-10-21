//! System preferences are read once, without spawning a helper process.
use crate::Language;

pub(super) fn detect() -> Language {
    if let Ok(value) = std::env::var("GW_LANG") {
        return match value.as_str() {
            "zh" => Language::Chinese,
            "en" => Language::English,
            _ => panic!("GW_LANG must be en or zh"),
        };
    }
    for key in ["LC_ALL", "LC_MESSAGES"] {
        if let Ok(locale) = std::env::var(key)
            && !locale.is_empty()
        {
            return Language::from_locale(&locale);
        }
    }
    if let Some(locale) = preferred_language() {
        return Language::from_locale(&locale);
    }
    Language::from_locale(&std::env::var("LANG").unwrap_or_default())
}

#[cfg(target_os = "macos")]
fn preferred_language() -> Option<String> {
    use std::ffi::{c_char, c_void};
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(array: *const c_void) -> isize;
        fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut c_char,
            size: isize,
            encoding: u32,
        ) -> bool;
        fn CFRelease(value: *const c_void);
    }
    // Copy 返回拥有的引用；数组内的字符串为借用引用，只释放数组。
    unsafe {
        let languages = CFLocaleCopyPreferredLanguages();
        if languages.is_null() {
            return None;
        }
        let result = if CFArrayGetCount(languages) > 0 {
            let first = CFArrayGetValueAtIndex(languages, 0);
            let mut buffer = [0u8; 256];
            if CFStringGetCString(
                first,
                buffer.as_mut_ptr().cast(),
                buffer.len() as isize,
                0x0800_0100,
            ) {
                Some(
                    std::ffi::CStr::from_ptr(buffer.as_ptr().cast())
                        .to_str()
                        .expect("CoreFoundation returned invalid UTF-8")
                        .to_owned(),
                )
            } else {
                panic!("System language identifier exceeds 255 UTF-8 bytes")
            }
        } else {
            None
        };
        CFRelease(languages);
        result
    }
}
#[cfg(not(target_os = "macos"))]
fn preferred_language() -> Option<String> {
    None
}
