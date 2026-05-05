#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::{anyhow, Result};

/// Resolve the on-disk path of the shared library (or test binary) that
/// contains this function. Used by the self-update path to locate the loaded
/// loader binary so we can rewrite the same file ClassiCube `dlopen`ed,
/// regardless of whether it lives at `plugins/...` or `plugins/managed/...`.
///
/// Linux/macOS: `dladdr` resolves any code address to its containing object.
/// Windows: `GetModuleHandleExW(FROM_ADDRESS, ...)` then `GetModuleFileNameW`.
#[cfg(unix)]
pub fn current_lib_path() -> Result<PathBuf> {
    use std::{ffi::CStr, mem, os::raw::c_void};

    let mut info: libc::Dl_info = unsafe { mem::zeroed() };
    let addr = current_lib_path as *const c_void;
    let rc = unsafe { libc::dladdr(addr, &mut info) };
    if rc == 0 {
        return Err(anyhow!("dladdr failed for current cdylib"));
    }
    if info.dli_fname.is_null() {
        return Err(anyhow!("dladdr returned null dli_fname"));
    }
    let cstr = unsafe { CStr::from_ptr(info.dli_fname) };
    let s = cstr
        .to_str()
        .map_err(|e| anyhow!("non-UTF8 dli_fname: {e}"))?;
    Ok(PathBuf::from(s))
}

#[cfg(windows)]
pub fn current_lib_path() -> Result<PathBuf> {
    use std::{
        ffi::OsString,
        os::{raw::c_void, windows::ffi::OsStringExt},
    };

    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::HMODULE,
            System::LibraryLoader::{
                GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            },
        },
    };

    let mut module = HMODULE::default();
    let addr = current_lib_path as *const c_void;
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(addr.cast::<u16>()),
            &mut module,
        )
        .map_err(|e| anyhow!("GetModuleHandleExW failed: {e}"))?;
    }

    let mut buf: Vec<u16> = vec![0; 1024];
    loop {
        let n = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
        if n == 0 {
            return Err(anyhow!("GetModuleFileNameW failed"));
        }
        if n < buf.len() {
            buf.truncate(n);
            break;
        }
        buf.resize(buf.len() * 2, 0);
    }
    Ok(PathBuf::from(OsString::from_wide(&buf)))
}
