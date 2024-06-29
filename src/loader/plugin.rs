use std::{cell::Cell, ffi::CString, fs, os::raw::c_void, path::Path};

use classicube_helpers::time;
use classicube_sys::{DynamicLib_Get2, DynamicLib_Load2, IGameComponent, OwnedString};

use crate::{
    cef_binary_updater::CEF_BINARY_PATH,
    error::*,
    plugin_updater::{CEF_EXE_PATH, CEF_PLUGIN_PATH},
};

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

    let ptr = unsafe { DynamicLib_Load2(path.as_cc_string()) };
    if ptr.is_null() {
        return Err(get_error().into());
    }

    Ok(ptr)
}

fn dll_get(library: *mut c_void, symbol_name: &str) -> Result<*mut c_void> {
    let symbol_name = CString::new(symbol_name)?;

    let ptr = unsafe { DynamicLib_Get2(library, symbol_name.as_ptr()) };
    if ptr.is_null() {
        return Err(get_error().into());
    }

    Ok(ptr)
}

pub fn try_init() -> Result<*mut IGameComponent> {
    #[cfg(target_os = "windows")]
    {
        use std::env;

        // copy cef-windows-x86_64.exe to cef.exe
        if let Err(e) = fs::copy(
            CEF_EXE_PATH,
            Path::new(CEF_EXE_PATH).parent().unwrap().join("cef.exe"),
        ) {
            tracing::warn!("couldn't copy cef exe: {}", e);
        }

        // add cef/cef_binary and cef/ to PATH so that cef.dll is found,
        // and cef.exe can run
        let path = env::var("PATH").unwrap_or_default();
        env::set_var("PATH", format!("{};{};{}", path, CEF_BINARY_PATH, "cef"));
    }

    #[cfg(target_os = "linux")]
    {
        use std::{env, os::unix::fs::PermissionsExt};

        // make it executable
        let mut perms = fs::metadata(CEF_EXE_PATH)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(CEF_EXE_PATH, perms)?;

        // copy cef-linux-x86_64 to cef
        let new_exe_path = Path::new(CEF_EXE_PATH).parent().unwrap().join("cef");
        if let Err(e) = fs::copy(CEF_EXE_PATH, new_exe_path) {
            tracing::warn!("couldn't copy cef exe: {}", e);
        }

        // add cef/cef_binary to LD_LIBRARY_PATH so that libcef.so is found
        if let Ok(ld_library_path) = env::var("LD_LIBRARY_PATH") {
            env::set_var(
                "LD_LIBRARY_PATH",
                format!("{}:{}", ld_library_path, CEF_BINARY_PATH),
            );
        } else {
            env::set_var("LD_LIBRARY_PATH", CEF_BINARY_PATH);
        }

        // add ./cef/ to path so that we can run "cef"
        let path = env::var("PATH").unwrap_or_default();
        env::set_var("PATH", format!("{}:{}", path, "cef"));

        // fix linux keyboard language layout mapping
        // on finnish keyboard: US [ is typed as ¥, but is suppose to be å
        env::set_var("LC_CTYPE", "C");
    }

    #[cfg(target_os = "macos")]
    {
        use std::{env, io::Write, os::unix::fs::PermissionsExt};

        // make it executable
        let mut perms = fs::metadata(CEF_EXE_PATH)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(CEF_EXE_PATH, perms)?;

        // trying to link with dlopen will just hang the window
        let dll_path = Path::new(CEF_BINARY_PATH).join("Chromium Embedded Framework");
        if !fs::metadata(&dll_path)
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            return Err("cef-binary missing".into());
        }

        // cef, cef (GPU), cef (Plugin), cef (Renderer)
        // net.classicube.game.cef, .cef.gpu, etc

        for (app_name, app_identifier) in &[
            ("cef", "com.classicube.game.cef"),
            ("cef (GPU)", "com.classicube.game.cef.gpu"),
            ("cef (Plugin)", "com.classicube.game.cef.plugin"),
            ("cef (Renderer)", "com.classicube.game.cef.renderer"),
        ] {
            fs::create_dir_all(format!("./cef/{}.app/Contents/MacOS", app_name))?;
            fs::copy(
                CEF_EXE_PATH,
                format!("./cef/{}.app/Contents/MacOS/{}", app_name, app_name),
            )?;

            let mut f = fs::File::create(format!("./cef/{}.app/Contents/Info.plist", app_name))?;
            write!(
                f,
                "{}",
                MAC_INFO_TEMPLATE
                    .replace("APP_NAME", app_name)
                    .replace("APP_IDENTIFIER", app_identifier)
            )?;
        }

        // add "cef/Chromium Embedded Framework.framework/Libraries" to DYLD_LIBRARY_PATH
        // so that libGLESv2.dylib/libEGL.dylib/libvk_swiftshader.dylib are found
        // ERROR:gl_implementation.cc(501)] Failed to load /cc/cef/cef.app/Contents/MacOS/libGLESv2.dylib:
        // dlopen(/cc/cef/cef.app/Contents/MacOS/libGLESv2.dylib, 0x0001):
        // tried: '/cc/cef/cef.app/Contents/MacOS/libGLESv2.dylib' (no such file)
        let libraries_path = format!("{}/Libraries", CEF_BINARY_PATH);
        if let Ok(dyld_library_path) = env::var("DYLD_LIBRARY_PATH") {
            env::set_var(
                "DYLD_LIBRARY_PATH",
                format!("{}:{}", dyld_library_path, libraries_path),
            );
        } else {
            env::set_var("DYLD_LIBRARY_PATH", format!("{}", libraries_path));
        }
    }

    let library = time!("dll_load", 5000, {
        tracing::debug!("dll_load {}", CEF_PLUGIN_PATH);
        dll_load(CEF_PLUGIN_PATH)?
    });
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

#[cfg(target_os = "macos")]
const MAC_INFO_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>APP_NAME</string>
	<key>CFBundleExecutable</key>
	<string>APP_NAME</string>
	<key>CFBundleIdentifier</key>
	<string>APP_IDENTIFIER</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>APP_NAME</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleSignature</key>
	<string>????</string>
	<key>LSEnvironment</key>
	<dict>
		<key>MallocNanoZone</key>
		<string>0</string>
	</dict>
	<key>LSFileQuarantineEnabled</key>
	<true/>
	<key>LSMinimumSystemVersion</key>
	<string>10.9.0</string>
	<key>LSUIElement</key>
	<string>1</string>
	<key>NSSupportsAutomaticGraphicsSwitching</key>
	<true/>
</dict>
</plist>"#;
