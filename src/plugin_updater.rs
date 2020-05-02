use crate::{
    cef_binary_updater, github_release_checker::GitHubReleaseChecker, print_async, AsyncManager,
};
use classicube_helpers::color;
use std::fs;

// windows 64 bit

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = r"plugins\classicube_cef_loader_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_PLUGIN_PATH: &str = r"cef\classicube_cef_windows_x86_64.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
pub const CEF_EXE_PATH: &str = r"cef\cef-windows-x86_64.exe";

// windows 32 bit

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_PLUGIN_LOADER_PATH: &str = r"plugins\classicube_cef_loader_windows_i686.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_PLUGIN_PATH: &str = r"cef\classicube_cef_windows_i686.dll";

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
pub const CEF_EXE_PATH: &str = r"cef\cef-windows-i686.exe";

pub fn update_plugins() {
    fs::create_dir_all("cef").unwrap();

    cef_binary_updater::prepare();

    AsyncManager::spawn(async move {
        let mut had_updates = false;

        let loader_plugin = GitHubReleaseChecker::new(
            "Cef Loader".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-loader-plugin".to_string(),
            vec![CEF_PLUGIN_LOADER_PATH.into()],
        );

        match loader_plugin.check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!(
                    "{}Failed to update: {}{}",
                    classicube_helpers::color::RED,
                    classicube_helpers::color::WHITE,
                    e
                ))
                .await;
            }
        }

        let cef_plugin = GitHubReleaseChecker::new(
            "Cef".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-plugin".to_string(),
            vec![CEF_PLUGIN_PATH.into(), CEF_EXE_PATH.into()],
        );

        match cef_plugin.check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!(
                    "{}Failed to update: {}{}",
                    classicube_helpers::color::RED,
                    classicube_helpers::color::WHITE,
                    e
                ))
                .await;
            }
        }

        match cef_binary_updater::check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!(
                    "{}Failed to update: {}{}",
                    classicube_helpers::color::RED,
                    classicube_helpers::color::WHITE,
                    e
                ))
                .await;
            }
        }

        if had_updates {
            print_async(format!(
                "{}Everything done, restart your game to finish the update!",
                color::YELLOW
            ))
            .await;
        }

        // AsyncManager::spawn_on_main_thread(async {
        //     debug!("marked for shutdown");
        //     AsyncManager::mark_for_shutdown();
        // });
    });
}
