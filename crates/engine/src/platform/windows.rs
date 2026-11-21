//! Windows path and access-control primitives.
use std::{
    ffi::{OsStr, c_void},
    io::{self, ErrorKind},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW,
        },
        DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetTokenInformation,
        PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_TEMPORARY, GetFileAttributesW,
        INVALID_FILE_ATTRIBUTES, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        SetFileAttributesW,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub(crate) fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut units: Vec<_> = value.encode_wide().collect();
    if units.contains(&0) {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            i18n::tr!(FolderInvalidPath),
        ));
    }
    units.push(0);
    Ok(units)
}

pub(crate) fn shell_path(path: &Path) -> std::ffi::OsString {
    use std::path::{Component, Prefix};
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(_) => std::ffi::OsString::from_wide(&units[4..]),
            Prefix::VerbatimUNC(_, _) => {
                let mut plain = vec![u16::from(b'\\'); 2];
                plain.extend_from_slice(&units[8..]);
                std::ffi::OsString::from_wide(&plain)
            }
            _ => path.as_os_str().to_owned(),
        },
        _ => path.as_os_str().to_owned(),
    }
}

pub(crate) fn move_file(source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
    let source = wide(source.as_os_str())?;
    let destination = wide(destination.as_os_str())?;
    let attributes = unsafe { GetFileAttributesW(source.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(io::Error::last_os_error());
    }
    if attributes & FILE_ATTRIBUTE_TEMPORARY != 0 {
        // 保留只读等属性，同时移除 tempfile 的临时文件缓存语义。
        let attributes = attributes & !FILE_ATTRIBUTE_TEMPORARY;
        let persistent = if attributes == 0 {
            FILE_ATTRIBUTE_NORMAL
        } else {
            attributes
        };
        if unsafe { SetFileAttributesW(source.as_ptr(), persistent) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    let flags = MOVEFILE_WRITE_THROUGH
        | if overwrite {
            MOVEFILE_REPLACE_EXISTING
        } else {
            0
        };
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn user_sid() -> io::Result<String> {
    let mut token = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut size = 0;
    unsafe { GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut size) };
    if (size as usize) < std::mem::size_of::<TOKEN_USER>() {
        return Err(io::Error::last_os_error());
    }
    // GetTokenInformation 将 TOKEN_USER 及其 SID 写到同一块对齐的缓冲区。
    let mut user = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    let buffer = user.as_mut_ptr().cast();
    if unsafe { GetTokenInformation(token.as_raw_handle(), TokenUser, buffer, size, &mut size) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    let sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut string = null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut string) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // Win32 返回以 NUL 结尾的 SID 字符串，并要求用 LocalFree 释放。
    let sid = unsafe {
        let len = (0..).find(|&index| *string.add(index) == 0).unwrap();
        let text = String::from_utf16(std::slice::from_raw_parts(string, len));
        LocalFree(string.cast());
        text
    };
    sid.map_err(|error| io::Error::new(ErrorKind::InvalidData, error))
}

pub(crate) fn restrict_to_user(path: &Path, directory: bool) -> io::Result<()> {
    // 使用登录用户 SID。管理员启动安装时，低权限的定时任务仍可访问同一份状态。
    let inheritance = if directory { "OICI" } else { "" };
    let sid = user_sid()?;
    let acl = format!("D:P(A;{inheritance};FA;;;{sid})(A;{inheritance};FA;;;SY)");
    let acl = wide(OsStr::new(&acl))?;
    let path = wide(path.as_os_str())?;
    let mut descriptor: *mut c_void = null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            acl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl = null_mut();
        if unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        let error = unsafe {
            SetNamedSecurityInfoW(
                path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                dacl,
                null_mut(),
            )
        };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        Ok(())
    })();
    unsafe { LocalFree(descriptor) };
    result
}

pub(crate) fn powershell(
    script: &str,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
    cancel: &crate::cancel::Cancel,
) -> anyhow::Result<crate::process::Output> {
    use anyhow::Context;
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .context("SystemRoot is missing or is not absolute")?;
    let program = root.join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let script = format!(
        "$ErrorActionPreference = 'Stop'; [Console]::OutputEncoding = [Text.UTF8Encoding]::new(); {script}"
    );
    crate::process::run_with_environment(
        &program,
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
        None,
        cancel,
        std::time::Duration::from_secs(30),
        environment,
    )
}
