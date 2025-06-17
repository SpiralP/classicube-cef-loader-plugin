use std::{
    io,
    marker::Unpin,
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{bail, Error, Result};
use classicube_helpers::color;
use futures::{
    stream::{StreamExt, TryStreamExt},
    Stream,
};
use tokio::{
    fs::{self, File},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
};
use tracing::*;

use crate::{async_manager, print_async, status, updater::make_client};

#[cfg(not(all(target_os = "linux", target_arch = "x86")))]
macro_rules! cef_version {
    () => {
        "134.3.8+gfe66d80+chromium-134.0.6998.166"
    };
}

// Linux x86 32-bit builds are discontinued after version 101 (details)
// https://cef-builds.spotifycdn.com/index.html#linux32
#[cfg(all(target_os = "linux", target_arch = "x86"))]
macro_rules! cef_version {
    () => {
        "101.0.18+g367b4a0+chromium-101.0.4951.67"
    };
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
macro_rules! cef_arch {
    () => {
        "windows64"
    };
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
macro_rules! cef_arch {
    () => {
        "windows32"
    };
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
macro_rules! cef_arch {
    () => {
        "linux64"
    };
}

#[cfg(all(target_os = "linux", target_arch = "x86"))]
macro_rules! cef_arch {
    () => {
        "linux32"
    };
}

#[cfg(all(target_os = "linux", target_arch = "arm"))]
macro_rules! cef_arch {
    () => {
        "linuxarm"
    };
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
macro_rules! cef_arch {
    () => {
        "linuxarm64"
    };
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
macro_rules! cef_arch {
    () => {
        "macosx64"
    };
}

pub const CEF_VERSION: &str = concat!("cef_binary_", cef_version!(), "_", cef_arch!(), "_minimal");

#[cfg(not(target_os = "macos"))]
pub const CEF_BINARY_PATH: &str = "cef/cef_binary";

#[cfg(target_os = "macos")]
pub const CEF_BINARY_PATH: &str = "cef/Chromium Embedded Framework.framework";

pub const CEF_BINARY_VERSION_PATH: &str = "cef/cef_binary.txt";

async fn get_current_version() -> Option<String> {
    if let Ok(bytes) = fs::read(CEF_BINARY_VERSION_PATH).await {
        String::from_utf8(bytes).map(|s| s.trim().to_string()).ok()
    } else {
        None
    }
}

pub async fn update() -> Result<bool> {
    let missing = !Path::new(CEF_BINARY_PATH).is_dir();

    let needs_update = missing
        || get_current_version()
            .await
            .map(|cur| cur != CEF_VERSION)
            .unwrap_or(true);

    if needs_update {
        print_async(format!(
            "{}Updating {}CEF Binary {}to {}{}",
            color::PINK,
            color::LIME,
            color::PINK,
            color::GREEN,
            CEF_VERSION
        ))
        .await;

        if Path::new(CEF_BINARY_PATH).is_dir() {
            fs::remove_dir_all(CEF_BINARY_PATH).await?;
        }
        fs::create_dir_all(CEF_BINARY_PATH).await?;
        download(CEF_VERSION).await?;

        {
            // mark as updated
            let mut f = File::create(CEF_BINARY_VERSION_PATH).await?;
            f.write_all(CEF_VERSION.as_bytes()).await?;
        }

        print_async(format!("{}CEF Binary finished downloading", color::LIME)).await;

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

impl<R> io::Read for FuturesBlockOnReader<R>
where
    R: AsyncRead + Unpin,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        futures::executor::block_on(self.async_reader.read(buf))
    }
}

async fn download(version: &str) -> Result<()> {
    let url = format!("https://cef-builds.spotifycdn.com/{}.tar.bz2", version).replace('+', "%2B");

    debug!("{}", url);

    let running = Arc::new(AtomicBool::new(true));
    let downloaded = Arc::new(AtomicUsize::new(0usize));
    let response = make_client().get(&url).send().await?;

    let maybe_content_length = response.content_length();

    if let Some(content_length) = maybe_content_length {
        print_async(format!(
            "{}Downloading {}{} {}({}{}MB{})",
            color::GOLD,
            //
            color::GREEN,
            "CEF Binary",
            color::GOLD,
            //
            color::GREEN,
            (content_length as f32 / 1024f32 / 1024f32).ceil() as u32,
            color::GOLD,
        ))
        .await;
    }

    {
        let running = Arc::downgrade(&running);
        let downloaded = downloaded.clone();

        async_manager::spawn_on_main_thread(async move {
            while let Some(running) = running.upgrade() {
                if !running.load(Ordering::SeqCst) {
                    debug!("status loop no longer running");
                    break;
                }

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

            debug!("status loop finished");
        });
    }

    let stream: Pin<Box<dyn Stream<Item = io::Result<_>> + Send>> = response
        .bytes_stream()
        .inspect(move |result| {
            if let Ok(bytes) = result {
                let len = bytes.len();
                downloaded.fetch_add(len, Ordering::SeqCst);
            }
        })
        .map_err(io::Error::other)
        .boxed();

    let stream = tokio_util::io::StreamReader::new(stream);

    let stream = tokio::io::BufReader::new(stream);

    let decoder = async_compression::tokio::bufread::BzDecoder::new(stream);

    let decoder = tokio::io::BufReader::new(decoder);

    let bad_reader = FuturesBlockOnReader {
        async_reader: decoder,
    };

    tokio::task::spawn_blocking(move || {
        let mut archive = tar::Archive::new(bad_reader);

        let mut cef_binary_name: Option<String> = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.clone();
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
                if first_part == "README.txt" || first_part == "LICENSE.txt" {
                    let out_path = Path::new(CEF_BINARY_PATH).join(first_part);
                    debug!("{:?} {:?}", path, out_path);

                    std::fs::create_dir_all(out_path.parent().unwrap())?;
                    file.unpack(&out_path)?;
                    continue;
                }

                // windows/linux extract files to cef/cef_binary/
                #[cfg(not(target_os = "macos"))]
                {
                    if let Some(ext) = trimmed_path.extension() {
                        if (first_part == "Release"
                            && (ext == "dll" || ext == "bin" || ext == "so"))
                            || (first_part == "Resources" && (ext == "pak" || ext == "dat"))
                        {
                            let even_more_trimmed: PathBuf = trimmed_path_components.collect();
                            // icu .dat and .bin files must be next to cef.dll
                            let out_path = Path::new(CEF_BINARY_PATH).join(even_more_trimmed);
                            debug!("{:?} {:?}", path, out_path);

                            std::fs::create_dir_all(out_path.parent().unwrap())?;
                            file.unpack(&out_path)?;

                            if ext == "so" {
                                debug!("stripping {:?}", out_path);
                                if let Ok(output) =
                                    std::process::Command::new("strip").arg(&out_path).output()
                                {
                                    if !output.status.success() {
                                        error!(
                                            "strip {:?}\n--- stdout\n{}\n--- stderr\n{}",
                                            out_path,
                                            String::from_utf8_lossy(&output.stdout),
                                            String::from_utf8_lossy(&output.stderr)
                                        );
                                        bail!("couldn't strip {:?}", out_path);
                                    }
                                }
                            }
                        }
                    }
                }

                // extract "Chromium Embedded Framework.framework" to "cef/Chromium Embedded Framework.framework"
                #[cfg(target_os = "macos")]
                {
                    if first_part == "Release" {
                        if let Some(Component::Normal(second_part)) = trimmed_path_components.next()
                        {
                            if second_part == "Chromium Embedded Framework.framework" {
                                let even_more_trimmed: PathBuf = trimmed_path_components.collect();
                                let out_path = Path::new(CEF_BINARY_PATH).join(&even_more_trimmed);
                                debug!("{:?} {:?}", path, out_path);

                                std::fs::create_dir_all(&out_path.parent().unwrap())?;
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

macro_rules! test_noop {
    ($name:tt) => {
        #[cfg(test)]
        #[no_mangle]
        pub extern "C" fn $name() {}
    };
}

test_noop!(Chat_AddOf);
test_noop!(Chat_Add);
test_noop!(ScheduledTask_Add);

#[test]
#[ignore]
fn test_update() {
    crate::logger::initialize(true, false);
    crate::async_manager::initialize();

    std::fs::create_dir_all("cef").unwrap();
    crate::async_manager::block_on_local(async {
        crate::async_manager::spawn(async {
            assert!(update().await.unwrap());
        })
        .await
        .unwrap();
    });

    crate::async_manager::shutdown();

    std::fs::remove_dir_all("cef").unwrap();
}
