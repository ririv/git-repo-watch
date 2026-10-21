//! Read the current executable-file handler from macOS Launch Services.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CStr, c_char, c_void},
    ptr::NonNull,
};

const UTF8: u32 = 0x0800_0100;
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        text: *const c_char,
        encoding: u32,
    ) -> *mut c_void;
    fn CFStringGetLength(value: *const c_void) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(
        value: *const c_void,
        buffer: *mut c_char,
        capacity: isize,
        encoding: u32,
    ) -> bool;
    fn CFRelease(value: *const c_void);
}
#[link(name = "CoreServices", kind = "framework")]
unsafe extern "C" {
    fn LSCopyDefaultRoleHandlerForContentType(
        content_type: *const c_void,
        roles: u32,
    ) -> *mut c_void;
}

struct OwnedString(NonNull<c_void>);
impl Drop for OwnedString {
    fn drop(&mut self) {
        // Create/Copy 返回的引用由当前封装唯一持有。
        unsafe {
            CFRelease(self.0.as_ptr());
        }
    }
}

pub(super) fn default_application() -> Result<String> {
    // 系统的可执行文件默认关联与文件夹的默认 Finder 关联是两种不同设置。
    let content_type = OwnedString(
        NonNull::new(unsafe {
            CFStringCreateWithCString(std::ptr::null(), c"public.unix-executable".as_ptr(), UTF8)
        })
        .context(i18n::tr!(ReadDefaultTerminalFailed))?,
    );
    let application = OwnedString(
        NonNull::new(unsafe {
            LSCopyDefaultRoleHandlerForContentType(content_type.0.as_ptr(), u32::MAX)
        })
        .context(i18n::tr!(DefaultTerminalNotConfigured))?,
    );
    let capacity = unsafe {
        CFStringGetMaximumSizeForEncoding(CFStringGetLength(application.0.as_ptr()), UTF8)
    };
    ensure!(capacity >= 0, i18n::tr!(ReadDefaultTerminalFailed));
    let mut bytes = vec![0u8; capacity as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            application.0.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len() as isize,
            UTF8,
        )
    };
    ensure!(converted, i18n::tr!(ReadDefaultTerminalFailed));
    let name = CStr::from_bytes_until_nul(&bytes)?.to_str()?;
    ensure!(!name.is_empty(), i18n::tr!(DefaultTerminalNotConfigured));
    Ok(name.to_owned())
}
