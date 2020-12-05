use crate::{error::*, print_async};
use classicube_helpers::color;
use futures::stream::TryStreamExt;
use log::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::{fs, io};

const VERSIONS_DIR_PATH: &str = "cef";

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub struct GitHubReleaseChecker {
    name: String,
    owner: String,
    repo: String,
    asset_paths: Vec<PathBuf>,
}

impl GitHubReleaseChecker {
    pub fn new<S: Into<String>, P: Into<Vec<PathBuf>>>(
        name: S,
        owner: S,
        repo: S,
        asset_paths: P,
    ) -> Self {
        Self {
            name: name.into(),
            owner: owner.into(),
            repo: repo.into(),
            asset_paths: asset_paths.into(),
        }
    }

    fn version_path(&self) -> PathBuf {
        let versions_dir = Path::new(VERSIONS_DIR_PATH);
        versions_dir.join(format!("{}.txt", self.repo))
    }

    fn url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        )
    }

    async fn get_current_version(&self) -> Option<String> {
        if let Ok(bytes) = fs::read(&self.version_path()).await {
            String::from_utf8(bytes).ok()
        } else {
            None
        }
    }

    fn make_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .unwrap()
    }

    pub async fn get_latest_release(&self) -> Result<GitHubRelease> {
        let client = Self::make_client();

        let release: GitHubRelease = client.get(&self.url()).send().await?.json().await?;
        if let Some(err) = release.message {
            Err(err.into())
        } else {
            Ok(release)
        }
    }

    pub async fn update(&self) -> Result<bool> {
        debug!("checking {:?}", self.name);

        // delete "-old" files
        // and check if we are missing any assets
        let mut missing_asset = false;
        for asset_path in &self.asset_paths {
            let asset_name = asset_path.file_name().unwrap().to_str().unwrap();

            let parent = asset_path.parent().unwrap();
            let wanted_path = Path::new(&parent).join(&asset_name);
            let old_path = Path::new(&parent).join(format!("{}-old", &asset_name));

            if let Err(e) = fs::remove_file(&old_path).await {
                // don't show error
                if e.kind() != io::ErrorKind::NotFound {
                    warn!("couldn't remove {:?}: {:#?}", &old_path, e);
                }
            }

            if !wanted_path.exists() {
                debug!("missing {:?}", wanted_path);
                missing_asset = true;
            }
        }

        let release = self.get_latest_release().await?;

        let needs_update = missing_asset
            || self
                .get_current_version()
                .await
                .map(|cur| cur != release.published_at)
                .unwrap_or(true);

        if needs_update {
            print_async(format!(
                "{}New release update {}{} {}for {}{}!",
                color::PINK,
                color::GREEN,
                release.tag_name,
                color::PINK,
                color::LIME,
                self.name
            ))
            .await;

            self.update_assets(&release).await?;

            {
                // mark that we updated
                fs::write(&self.version_path(), release.published_at).await?;
            }

            print_async(format!("{}{} finished downloading", color::LIME, self.name)).await;

            Ok(true)
        } else {
            debug!("{} up to date", self.name);
            Ok(false)
        }
    }

    async fn update_assets(&self, release: &GitHubRelease) -> Result<()> {
        for asset_path in &self.asset_paths {
            let asset_name = asset_path.file_name().unwrap().to_str().unwrap();

            let asset = release
                .assets
                .iter()
                .find(|asset| asset.name == asset_name)
                .chain_err(|| format!("couldn't find asset {}", asset_name))?;

            print_async(format!(
                "{}Downloading {}{} {}({}{}MB{})",
                color::GOLD,
                //
                color::GREEN,
                asset.name,
                color::GOLD,
                //
                color::GREEN,
                (asset.size as f32 / 1024f32 / 1024f32).ceil() as u32,
                color::GOLD,
            ))
            .await;

            let parent = asset_path.parent().unwrap();
            let wanted_path = Path::new(&parent).join(&asset_name);
            let new_path = Path::new(&parent).join(format!("{}-new", &asset_name));
            let old_path = Path::new(&parent).join(format!("{}-old", &asset_name));
            {
                let mut f = fs::File::create(&new_path).await?;

                let mut stream = io::stream_reader(
                    Self::make_client()
                        .get(&asset.browser_download_url)
                        .send()
                        .await?
                        .bytes_stream()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
                );

                io::copy(&mut stream, &mut f).await?;
            }

            if wanted_path.is_file() {
                // we need to flip/flop files

                // try to rename current loaded to -old
                if let Err(e) = fs::rename(&wanted_path, &old_path).await {
                    // if we can't rename to -old, it's probably still loaded
                    // and we're updating a second time,
                    // so try to delete current file which is probably not loaded
                    if let Err(e2) = fs::remove_file(&wanted_path).await {
                        bail!("failed to rename current file: {} and {}", e, e2);
                    } else {
                        debug!("deleted {:?} ok", &wanted_path);
                    }
                } else {
                    debug!("renamed {:?} -> {:?} ok", &wanted_path, &old_path);
                }
            }

            // rename downloaded to wanted_path
            fs::rename(&new_path, &wanted_path).await?;

            print_async(format!(
                "{}Finished downloading {}{}",
                color::GOLD,
                color::GREEN,
                asset.name,
            ))
            .await;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    /// error message
    pub message: Option<String>,

    pub tag_name: String,
    pub assets: Vec<GitHubReleaseAsset>,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReleaseAsset {
    pub browser_download_url: String,
    pub name: String,
    pub size: usize,
}

#[ignore]
#[tokio::test]
async fn test_github_release_checker() {
    let loader_plugin = GitHubReleaseChecker::new(
        "Cef Loader",
        "SpiralP",
        "classicube-cef-loader-plugin",
        vec![],
    );
    let release = loader_plugin.get_latest_release().await.unwrap();
    println!("{:#?}", release);
}
