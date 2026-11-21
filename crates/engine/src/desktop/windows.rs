//! Hand desktop opens to the Windows shell on the opener's worker thread.
use super::{Local, Target};
use crate::platform::windows::{shell_path, wide};
use anyhow::{Result, ensure};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
};

pub(super) fn open(target: &Target) -> Result<()> {
    let argument = match target {
        Target::Local(Local(path)) | Target::Directory(Local(path)) => shell_path(path),
        Target::Web(_) => target.argument(),
    };
    let path = wide(&argument)?;
    let explore: Vec<_> = "explore\0".encode_utf16().collect();
    let verb = match target {
        Target::Directory(_) => explore.as_ptr(),
        _ => null(),
    };

    // Shell 扩展可能需要单线程 COM 环境；打开操作在独立线程中执行。
    let initialized = unsafe {
        CoInitializeEx(
            null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    ensure!(initialized >= 0, "CoInitializeEx: {initialized:#010x}");
    let result = unsafe {
        let status = ShellExecuteW(
            null_mut(),
            verb,
            path.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
        CoUninitialize();
        status as isize
    };
    ensure!(result > 32, "ShellExecuteW: {} ({result})", target.label());
    Ok(())
}
