use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Error, Result};
use classicube_helpers::color;
use futures::stream::TryStreamExt;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use tokio::{fs, io};
use tracing::*;

use crate::{print_async, updater::make_client};

const VERSIONS_DIR_PATH: &str = "cef";

#[derive(Debug, Deserialize)]
struct GitHubError {
    message: String,
}

pub struct GitHubReleaseChecker {
    name: String,
    #[allow(dead_code)]
    owner: String,
    repo: String,
    asset_paths: Vec<PathBuf>,
    release: GitHubRelease,
}

impl GitHubReleaseChecker {
    pub async fn create<S: Into<String>, P: Into<Vec<PathBuf>>>(
        name: S,
        owner: S,
        repo: S,
        asset_paths: P,
    ) -> Result<Self> {
        let owner: String = owner.into();
        let repo: String = repo.into();
        let release = Self::get_latest_release(&owner, &repo).await?;

        Ok(Self {
            name: name.into(),
            owner,
            repo,
            asset_paths: asset_paths.into(),
            release,
        })
    }

    fn version_path(&self) -> PathBuf {
        let versions_dir = Path::new(VERSIONS_DIR_PATH);
        versions_dir.join(format!("{}.txt", self.repo))
    }

    async fn get_current_version(&self) -> Option<String> {
        if let Ok(bytes) = fs::read(&self.version_path()).await {
            String::from_utf8(bytes).map(|s| s.trim().to_string()).ok()
        } else {
            None
        }
    }

    async fn get_latest_release(owner: &str, repo: &str) -> Result<GitHubRelease> {
        let mut request = make_client().get(format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/latest"
        ));
        if let Ok(token) = env::var("GITHUB_TOKEN") {
            let mut header_value = HeaderValue::from_str(&format!("token {token}")).unwrap();
            header_value.set_sensitive(true);
            request = request.header(AUTHORIZATION, header_value);
        }

        let bytes = request.send().await?.bytes().await?;

        if let Ok(error) = serde_json::from_slice::<GitHubError>(&bytes) {
            bail!("{}", error.message);
        } else {
            Ok::<_, Error>(serde_json::from_slice::<GitHubRelease>(&bytes)?)
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
            let wanted_path = Path::new(&parent).join(asset_name);
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

        let needs_update = missing_asset
            || self
                .get_current_version()
                .await
                .map(|cur| cur != self.release.published_at)
                .unwrap_or(true);

        if needs_update {
            print_async(format!(
                "{}New release update {}{} {}for {}{}!",
                color::PINK,
                color::GREEN,
                self.release.tag_name,
                color::PINK,
                color::LIME,
                self.name
            ))
            .await;

            self.update_assets(&self.release).await?;

            {
                // mark that we updated
                fs::write(&self.version_path(), &self.release.published_at).await?;
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
                .with_context(|| format!("couldn't find asset {}", asset_name))?;

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
            let wanted_path = Path::new(&parent).join(asset_name);
            let new_path = Path::new(&parent).join(format!("{}-new", &asset_name));
            let old_path = Path::new(&parent).join(format!("{}-old", &asset_name));
            {
                let mut f = fs::File::create(&new_path).await?;

                let mut stream = tokio_util::io::StreamReader::new(
                    make_client()
                        .get(&asset.browser_download_url)
                        .send()
                        .await?
                        .error_for_status()?
                        .bytes_stream()
                        .map_err(io::Error::other),
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

    pub async fn get_file(&self, file_path: &str) -> Result<String> {
        let owner = &self.owner;
        let repo = &self.repo;
        let tag_name = &self.release.tag_name;

        let text = make_client()
            .get(format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/refs/tags/{tag_name}/{file_path}"
            ))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(text)
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    /// error message
    #[allow(dead_code)]
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
    let release = GitHubReleaseChecker::create(
        "Cef Loader",
        "SpiralP",
        "classicube-cef-loader-plugin",
        vec![],
    )
    .await
    .unwrap();
    println!("{:#?}", release.release);
}
