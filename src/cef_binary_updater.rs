use crate::{async_manager, error::*, print_async, status};
use classicube_helpers::color;
use futures::stream::{StreamExt, TryStreamExt};
use log::debug;
use std::{
    fs, io,
    io::{Read, Write},
    marker::Unpin,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::prelude::*;

macro_rules! cef_version {
    () => {
        "cef_binary_81.2.17+gb382c62+chromium-81.0.4044.113"
    };
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
macro_rules! cef_arch {
    () => {
        "windows64"
    };
}

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
macro_rules! cef_arch {
    () => {
        "windows32"
    };
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
macro_rules! cef_arch {
    () => {
        "linux64"
    };
}

#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
macro_rules! cef_arch {
    () => {
        "macosx64"
    };
}

const CEF_VERSION: &str = concat!(cef_version!(), "_", cef_arch!(), "_minimal");

#[cfg(not(target_os = "macos"))]
const CEF_BINARY_DIR_NAME: &str = "cef_binary";

#[cfg(target_os = "macos")]
const CEF_BINARY_DIR_NAME: &str = "Chromium Embedded Framework.framework";

const CEF_BINARY_VERSION_FILE_NAME: &str = "cef_binary.txt";

pub fn cef_binary_path() -> PathBuf {
    Path::new("cef").join(CEF_BINARY_DIR_NAME)
}

fn version_path() -> PathBuf {
    Path::new("cef").join(CEF_BINARY_VERSION_FILE_NAME)
}

fn get_current_version() -> Option<String> {
    fs::File::open(version_path())
        .map(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        })
        .ok()
}

pub async fn check() -> Result<bool> {
    let current_version = get_current_version().unwrap_or_default();

    if current_version != CEF_VERSION {
        print_async(format!(
            "{}Updating {}cef-binary {}to {}{}",
            color::PINK,
            color::LIME,
            color::PINK,
            color::GREEN,
            CEF_VERSION
        ))
        .await;

        let parent = Path::new("cef");
        let wanted_path = Path::new(&parent).join(CEF_BINARY_DIR_NAME);
        let new_path = Path::new(&parent).join(format!("{}-new", CEF_BINARY_DIR_NAME));

        fs::create_dir_all(&new_path)?;
        download(CEF_VERSION, new_path.to_path_buf()).await?;

        // remove old dir
        if Path::new(&wanted_path).is_dir() {
            fs::remove_dir_all(&wanted_path)?;
        }

        // rename downloaded to wanted_path, replaces if existed
        fs::rename(&new_path, &wanted_path)?;

        {
            // mark that we updated
            let mut f = fs::File::create(version_path())?;
            write!(f, "{}", CEF_VERSION)?;
        }

        print_async(format!("{}cef-binary finished downloading", color::LIME)).await;

        Ok(true)
    } else {
        Ok(false)
    }
}

struct FuturesBlockOnReader<R>
where
    R: AsyncRead,
{
    async_reader: R,
}

impl<R> Read for FuturesBlockOnReader<R>
where
    R: AsyncRead + Unpin,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        futures::executor::block_on(self.async_reader.read(buf))
    }
}

async fn download(version: &str, cef_binary_dir_path: PathBuf) -> Result<()> {
    use futures::compat::{Compat, Compat01As03};
    use tokio_util::compat::{FuturesAsyncReadCompatExt, Tokio02AsyncReadCompatExt};

    let url = format!(
        "http://opensource.spotify.com/cefbuilds/{}.tar.bz2",
        version
    )
    .replace("+", "%2B");

    let running = Arc::new(AtomicBool::new(true));
    let downloaded = Arc::new(AtomicUsize::new(0usize));
    let response = reqwest::get(&url).await?;

    {
        let maybe_content_length = response.content_length();
        let running = running.clone();
        let downloaded = downloaded.clone();

        async_manager::spawn_on_main_thread(async move {
            while running.load(Ordering::SeqCst) {
                let downloaded = downloaded.load(Ordering::SeqCst);

                let message = if let Some(content_length) = maybe_content_length {
                    format!(
                        "{:.2}%",
                        (downloaded as f32 / content_length as f32) * 100.0
                    )
                } else {
                    format!("{} bytes", downloaded)
                };

                status(format!(
                    "{}Downloading ({}{}{})",
                    color::PINK,
                    color::LIME,
                    message,
                    color::PINK,
                ));

                async_manager::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    let stream = response
        .bytes_stream()
        .inspect(move |result| {
            if let Ok(bytes) = result {
                let len = bytes.len();
                downloaded.fetch_add(len, Ordering::SeqCst);
            }
        })
        .boxed();

    let stream =
        tokio::io::stream_reader(stream.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

    let stream = tokio::io::BufReader::new(stream);
    let stream = Tokio02AsyncReadCompatExt::compat(stream);

    let stream = Compat::new(stream);
    let decoder = bzip2::read::BzDecoder::new(stream);
    let decoder = Compat01As03::new(decoder);

    let decoder = FuturesAsyncReadCompatExt::compat(decoder);
    let decoder = tokio::io::BufReader::new(decoder);

    let bad_reader = FuturesBlockOnReader {
        async_reader: decoder,
    };

    tokio::task::spawn_blocking(move || {
        let mut archive = tar::Archive::new(bad_reader);

        let mut cef_binary_name: Option<String> = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_owned();
            let mut components = path.components();

            // remove cef_binary_* part
            let first_component = if let Some(Component::Normal(component)) = components.next() {
                component.to_str().unwrap().to_string()
            } else {
                unreachable!();
            };

            // check we always have the same first directory
            if let Some(cef_binary_name) = &cef_binary_name {
                assert!(cef_binary_name == &first_component);
            } else {
                cef_binary_name = Some(first_component);
            }

            let trimmed_path: PathBuf = components
                .inspect(|part| {
                    if let Component::Normal(_) = part {
                    } else {
                        // don't allow anything but Normal
                        unreachable!();
                    }
                })
                .collect();

            let mut trimmed_path_components = trimmed_path.components();

            if let Some(Component::Normal(first_part)) = trimmed_path_components.next() {
                #[cfg(not(target_os = "macos"))]
                {
                    if let Some(ext) = trimmed_path.extension() {
                        if (first_part == "Release"
                            && (ext == "dll" || ext == "bin" || ext == "so"))
                            || (first_part == "Resources" && (ext == "pak" || ext == "dat"))
                        {
                            let even_more_trimmed: PathBuf = trimmed_path_components.collect();
                            // icu .dat and .bin files must be next to cef.dll
                            let out_path = Path::new(&cef_binary_dir_path).join(&even_more_trimmed);
                            debug!("{:?} {:?}", path, out_path);

                            let parent = out_path.parent().unwrap();
                            fs::create_dir_all(&parent)?;
                            file.unpack(&out_path)?;
                        }
                    }
                }

                // mac needs to extract Chromium Embedded Framework.framework to cef/
                #[cfg(target_os = "macos")]
                {
                    if first_part == "Release" {
                        if let Some(Component::Normal(second_part)) = trimmed_path_components.next()
                        {
                            if second_part == "Chromium Embedded Framework.framework" {
                                let even_more_trimmed: PathBuf = trimmed_path_components.collect();
                                let out_path =
                                    Path::new(&cef_binary_dir_path).join(&even_more_trimmed);
                                debug!("{:?} {:?}", path, out_path);

                                let parent = out_path.parent().unwrap();
                                fs::create_dir_all(&parent)?;
                                file.unpack(&out_path)?;
                            }
                        }
                    }
                }
            }
        }

        Ok::<(), Error>(())
    })
    .await??;

    running.store(false, Ordering::SeqCst);

    async_manager::run_on_main_thread(async {
        status("");
    })
    .await;

    Ok(())
}
