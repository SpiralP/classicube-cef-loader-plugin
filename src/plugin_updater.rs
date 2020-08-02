use crate::{
    cef_binary_updater, error::*, github_release_checker::GitHubReleaseChecker, print_async,
};
use classicube_helpers::color;
use futures::{prelude::*, stream};

// windows 64 bit

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const CEF_PLUGIN_PATH: &str = "cef/classicube_cef_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-windows-x86_64.exe";

// windows 32 bit

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_windows_i686.dll";

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub const CEF_PLUGIN_PATH: &str = "cef/classicube_cef_windows_i686.dll";

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub const CEF_EXE_PATH: &str = "cef/cef-windows-i686.exe";

// linux 64 bit

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_linux_x86_64.so";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_linux_x86_64.so";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-linux-x86_64";

// linux 32 bit

#[cfg(all(target_os = "linux", target_arch = "x86"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_linux_i686.so";

#[cfg(all(target_os = "linux", target_arch = "x86"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_linux_i686.so";

#[cfg(all(target_os = "linux", target_arch = "x86"))]
pub const CEF_EXE_PATH: &str = "cef/cef-linux-i686";

// linux armhf

#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_linux_armhf.so";

#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_linux_armhf.so";

#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub const CEF_EXE_PATH: &str = "cef/cef-linux-armhf";

// linux aarch64

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_linux_aarch64.so";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_linux_aarch64.so";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-linux-aarch64";

// macos 64 bit

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_macos_x86_64.dylib";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_macos_x86_64.dylib";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-macos-x86_64";

fn plugin_futures() -> Vec<impl Future<Output = Result<bool>>> {
    macro_rules! add {
        ($future:expr) => {{
            async { $future.await }.boxed()
        }};
    }

    vec![
        add!(GitHubReleaseChecker::new(
            "Cef Loader",
            "SpiralP",
            "classicube-cef-loader-plugin",
            vec![CEF_PLUGIN_LOADER_PATH.into()],
        )
        .update()),
        add!(GitHubReleaseChecker::new(
            "Cef",
            "SpiralP",
            "classicube-cef-plugin",
            vec![CEF_PLUGIN_PATH.into(), CEF_EXE_PATH.into()],
        )
        .update()),
        add!(cef_binary_updater::check()),
    ]
}

pub async fn update_plugins() -> Result<()> {
    let mut had_updates = false;

    let results = stream::iter(plugin_futures())
        .buffer_unordered(4)
        .collect::<Vec<Result<bool>>>()
        .await;

    for result in results {
        let updated = result?;
        if updated {
            had_updates = true;
        }
    }

    if had_updates {
        print_async(format!(
            "{}Everything done, restart your game to finish the update!",
            color::YELLOW
        ))
        .await;
    }

    Ok(())
}
