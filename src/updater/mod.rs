pub mod cef_binary;
pub mod github_release;

use std::time::Duration;

use anyhow::Result;
use github_release::GitHubReleaseChecker;

pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

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

pub async fn update_plugins() -> Result<()> {
    let cef_loader_plugin_updated = GitHubReleaseChecker::create(
        "CEF Loader Plugin",
        "SpiralP",
        "classicube-cef-loader-plugin",
        vec![CEF_PLUGIN_LOADER_PATH.into()],
    )
    .await?
    .update()
    .await?;

    if cef_loader_plugin_updated {
        // TODO should we break if cef loader plugin updated?
    }

    let cef_plugin_release = GitHubReleaseChecker::create(
        "CEF Plugin",
        "SpiralP",
        "classicube-cef-plugin",
        vec![CEF_PLUGIN_PATH.into(), CEF_EXE_PATH.into()],
    )
    .await?;

    let cef_binary_version = if cfg!(all(target_os = "linux", target_arch = "x86")) {
        // TODO this probably doesn't work anymore, since newer cef plugin requires matching cef_binary version
        // Linux x86 32-bit builds are discontinued after version 101
        // https://cef-builds.spotifycdn.com/index.html#linux32
        "101.0.18+g367b4a0+chromium-101.0.4951.67".to_string()
    } else {
        cef_plugin_release
            .get_file("cef_binary_version")
            .await?
            .trim()
            .to_string()
    };

    cef_plugin_release.update().await?;

    cef_binary::update(&cef_binary_version).await?;

    Ok(())
}
