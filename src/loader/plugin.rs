use crate::{
    cef_binary_updater::CEF_BINARY_PATH,
    error::*,
    plugin_updater::{CEF_EXE_PATH, CEF_PLUGIN_PATH},
};
use classicube_sys::{DynamicLib_Get, DynamicLib_Load, IGameComponent, OwnedString};
use std::{cell::Cell, env, ffi::CString, fs, os::raw::c_void, path::Path, ptr};

thread_local!(
    static LIBRARY: Cell<Option<*mut c_void>> = Cell::new(None);
);

fn get_error() -> String {
    #[cfg(windows)]
    {
        format!("{}", std::io::Error::last_os_error())
    }

    #[cfg(unix)]
    {
        let e = unsafe { std::ffi::CStr::from_ptr(libc::dlerror()) };
        e.to_string_lossy().to_string()
    }
}

fn dll_load(path: &str) -> Result<*mut c_void> {
    let path = OwnedString::new(path);

    let mut ptr = ptr::null_mut();
    if unsafe { DynamicLib_Load(path.as_cc_string(), &mut ptr) } != 0 {
        return Err(get_error().into());
    }

    Ok(ptr)
}

fn dll_get(library: *mut c_void, symbol_name: &str) -> Result<*mut c_void> {
    let symbol_name = CString::new(symbol_name)?;

    let mut ptr = ptr::null_mut();
    if unsafe { DynamicLib_Get(library, symbol_name.as_ptr(), &mut ptr) } != 0 {
        return Err(get_error().into());
    }

    Ok(ptr)
}

pub fn try_init() -> Result<*mut IGameComponent> {
    #[cfg(target_os = "windows")]
    {
        // copy cef-windows-x86_64.exe to cef.exe
        fs::copy(
            CEF_EXE_PATH,
            Path::new(CEF_EXE_PATH).parent().unwrap().join("cef.exe"),
        )?;

        // add cef/cef_binary and cef/ to PATH so that cef.dll is found,
        // and cef.exe can run
        let path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{};{};{}", path, CEF_BINARY_PATH, "cef"));
    }

    #[cfg(target_os = "linux")]
    {
        // copy cef-linux-x86_64 to cef
        fs::copy(
            CEF_EXE_PATH,
            Path::new(CEF_EXE_PATH).parent().unwrap().join("cef"),
        )?;

        // add cef/cef_binary to LD_LIBRARY_PATH so that libcef.so is found
        if let Ok(ld_library_path) = env::var("LD_LIBRARY_PATH") {
            env::set_var(
                "LD_LIBRARY_PATH",
                format!("{}:{}", ld_library_path, CEF_BINARY_PATH),
            );
        } else {
            env::set_var("LD_LIBRARY_PATH", format!("{}/", CEF_BINARY_PATH));
        }

        log::warn!("{:#?}", env::var("LD_LIBRARY_PATH"));

        // add ./cef/ to path so that we can run "cef"
        let path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", path, "cef"));
    }

    let library = dll_load(CEF_PLUGIN_PATH)?;
    LIBRARY.with(|cell| cell.set(Some(library)));

    let plugin_component = dll_get(library, "Plugin_Component")?;
    let plugin_component: *mut IGameComponent = plugin_component as _;

    Ok(plugin_component)
}

pub fn free() {
    LIBRARY.with(|cell| {
        if let Some(_library) = cell.get() {
            cell.set(None);
        }
    });
}
