use crate::{
    cef_binary_updater, error::*, github_release_checker::GitHubReleaseChecker, print_async,
};
use classicube_helpers::color;
use std::fs;

// windows 64 bit

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_PLUGIN_PATH: &str = "cef/classicube_cef_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-windows-x86_64.exe";

// windows 32 bit

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_windows_i686.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_PLUGIN_PATH: &str = "cef/classicube_cef_windows_i686.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_EXE_PATH: &str = "cef/cef-windows-i686.exe";

// linux 64 bit

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_linux_x86_64.so";

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_linux_x86_64.so";

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-linux-x86_64";

// macos 64 bit

#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = "plugins/classicube_cef_loader_macos_x86_64.dylib";

#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
pub const CEF_PLUGIN_PATH: &str = "./cef/classicube_cef_macos_x86_64.dylib";

#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
pub const CEF_EXE_PATH: &str = "cef/cef-macos-x86_64";

pub async fn update_plugins() -> Result<()> {
    fs::create_dir_all("cef").unwrap();

    cef_binary_updater::prepare();

    let mut had_updates = false;

    let loader_plugin = GitHubReleaseChecker::new(
        "Cef Loader".to_string(),
        "SpiralP".to_string(),
        "rust-classicube-cef-loader-plugin".to_string(),
        vec![CEF_PLUGIN_LOADER_PATH.into()],
    );
    let updated = loader_plugin.check().await?;
    if updated {
        had_updates = true;
    }

    let cef_plugin = GitHubReleaseChecker::new(
        "Cef".to_string(),
        "SpiralP".to_string(),
        "rust-classicube-cef-plugin".to_string(),
        vec![CEF_PLUGIN_PATH.into(), CEF_EXE_PATH.into()],
    );
    let updated = cef_plugin.check().await?;
    if updated {
        had_updates = true;
    }

    let updated = cef_binary_updater::check().await?;
    if updated {
        had_updates = true;
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
