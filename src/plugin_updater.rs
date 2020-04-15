use crate::{
    cef_binary_updater, github_release_checker::GitHubReleaseChecker, print_async, AsyncManager,
};
use std::fs;

pub const CEF_PLUGIN_PATH: &str = r"cef\classicube_cef_windows_amd64.dll";

pub fn update_plugins() {
    fs::create_dir_all("cef").unwrap();

    AsyncManager::spawn(async move {
        let updater_plugin = GitHubReleaseChecker::new(
            "Cef Loader".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-loader-plugin".to_string(),
            vec![r"plugins\classicube_cef_loader_windows_amd64.dll".into()],
        );

        if let Err(e) = updater_plugin.check().await {
            print_async(format!("Failed to check updates: {}", e)).await;
        }

        let updater_plugin = GitHubReleaseChecker::new(
            "Cef".to_string(),
            "SpiralP".to_string(),
            "rust-classicube-cef-plugin".to_string(),
            vec![CEF_PLUGIN_PATH.into(), r"cef\cefsimple.exe".into()],
        );

        if let Err(e) = updater_plugin.check().await {
            print_async(format!("Failed to check updates: {}", e)).await;
        }

        if let Err(e) = cef_binary_updater::check().await {
            print_async(format!("Failed to check updates: {}", e)).await;
        }
    });
}
