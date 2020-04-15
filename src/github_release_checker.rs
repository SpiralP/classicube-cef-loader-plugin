use crate::{error::*, print_async};
use futures::stream::TryStreamExt;
use serde::Deserialize;
use std::{
    fs, io,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const VERSIONS_DIR_PATH: &str = "cef";

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub struct GitHubReleaseChecker {
    name: String,
    owner: String,
    repo: String,
    asset_paths: Vec<PathBuf>,
}

impl GitHubReleaseChecker {
    pub fn new(name: String, owner: String, repo: String, asset_paths: Vec<PathBuf>) -> Self {
        Self {
            name,
            owner,
            repo,
            asset_paths,
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

    fn get_current_version(&self) -> Option<String> {
        fs::File::open(&self.version_path())
            .map(|mut f| {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s
            })
            .ok()
    }

    fn make_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .unwrap()
    }

    async fn get_latest_release(&self) -> Result<GitHubRelease> {
        let client = Self::make_client();

        Ok(client.get(&self.url()).send().await?.json().await?)
    }

    pub async fn check(&self) -> Result<()> {
        let current_version = self.get_current_version().unwrap_or_default();
        match self.get_latest_release().await {
            Ok(release) => {
                if current_version != release.published_at {
                    print_async(format!(
                        "New release update {} for {}!",
                        release.tag_name, self.name
                    ))
                    .await;

                    self.update_assets(&release).await?;

                    {
                        // mark that we updated
                        let mut f = fs::File::create(&self.version_path()).unwrap();
                        write!(f, "{}", release.published_at).unwrap();
                    }

                    print_async(format!(
                        "{} finished downloading, restart your game to finish the update!",
                        self.name
                    ))
                    .await;
                }
            }

            Err(e) => {
                print_async(format!("Couldn't get release for {}: {}", self.repo, e)).await;
            }
        }

        Ok(())
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
                "Downloading {} ({}MB)",
                asset.name,
                (asset.size as f32 / 1024f32 / 1024f32).ceil() as u32
            ))
            .await;

            let wanted_path = Path::new(&asset_path.parent().unwrap()).join(&asset.name);
            let new_path =
                Path::new(&asset_path.parent().unwrap()).join(format!("{}-new", &asset.name));
            let old_path =
                Path::new(&asset_path.parent().unwrap()).join(format!("{}-old", &asset.name));
            {
                let mut f = tokio::fs::File::create(&new_path).await?;

                let mut stream = tokio::io::stream_reader(
                    Self::make_client()
                        .get(&asset.browser_download_url)
                        .send()
                        .await?
                        .bytes_stream()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
                );

                tokio::io::copy(&mut stream, &mut f).await?;
            }

            if wanted_path.is_file() {
                // we need to flip/flop files

                // rename current loaded to -old
                fs::rename(&wanted_path, &old_path)?;
            }

            // rename downloaded to wanted_path
            fs::rename(&new_path, &wanted_path)?;

            print_async(format!("Finished downloading {}", asset.name)).await;
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubReleaseAsset>,
    published_at: String,
}

#[derive(Deserialize)]
struct GitHubReleaseAsset {
    browser_download_url: String,
    name: String,
    size: usize,
}
