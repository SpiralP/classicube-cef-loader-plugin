use crate::{
    cef_binary_updater, github_release_checker::GitHubReleaseChecker, print_async, AsyncManager,
};
use std::fs;

pub const CEF_PLUGIN_PATH: &str = r"cef\classicube_cef_windows_amd64.dll";

pub fn update_plugins() {
    fs::create_dir_all("cef").unwrap();

    cef_binary_updater::prepare();

    AsyncManager::spawn(async move {
        let mut had_updates = false;

        let loader_plugin = GitHubReleaseChecker::new(
            "Cef Loader".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-loader-plugin".to_string(),
            vec![r"plugins\classicube_cef_loader_windows_amd64.dll".into()],
        );

        match loader_plugin.check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!("Failed to update: {}", e)).await;
            }
        }

        let cef_plugin = GitHubReleaseChecker::new(
            "Cef".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-plugin".to_string(),
            vec![CEF_PLUGIN_PATH.into(), r"cef\cefsimple.exe".into()],
        );

        match cef_plugin.check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!("Failed to update: {}", e)).await;
            }
        }

        match cef_binary_updater::check().await {
            Ok(updated) => {
                if updated {
                    had_updates = true;
                }
            }

            Err(e) => {
                print_async(format!("Failed to update: {}", e)).await;
            }
        }

        if had_updates {
            print_async("Everything done, restart your game to finish the update!").await;
        }

        // AsyncManager::spawn_on_main_thread(async {
        //     println!("marked for shutdown");
        //     AsyncManager::mark_for_shutdown();
        // });
    });
}
