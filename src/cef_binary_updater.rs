use crate::{async_manager, error::*, print_async, status};
use classicube_helpers::color;
use futures::stream::{StreamExt, TryStreamExt};
use log::{debug, error};
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
        "cef_binary_84.3.3+gffc2f0e+chromium-84.0.4147.89"
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

pub const CEF_VERSION: &str = concat!(cef_version!(), "_", cef_arch!(), "_minimal");

#[cfg(not(target_os = "macos"))]
pub const CEF_BINARY_PATH: &str = "cef/cef_binary";

#[cfg(target_os = "macos")]
pub const CEF_BINARY_PATH: &str = "cef/Chromium Embedded Framework.framework";

#[cfg(not(target_os = "macos"))]
pub const CEF_BINARY_PATH_NEW: &str = "cef/cef_binary-new";

#[cfg(target_os = "macos")]
pub const CEF_BINARY_PATH_NEW: &str = "cef/Chromium Embedded Framework.framework-new";

pub const CEF_BINARY_VERSION_PATH: &str = "cef/cef_binary.txt";
pub const CEF_BINARY_VERSION_PATH_NEW: &str = "cef/cef_binary.txt-new";

fn get_current_version() -> Option<String> {
    fs::File::open(CEF_BINARY_VERSION_PATH)
        .map(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        })
        .ok()
}

pub fn prepare() -> Result<()> {
    // cef's .bin files are locked hard so we can't do the flip/flop
    if Path::new(CEF_BINARY_VERSION_PATH_NEW).is_file() && Path::new(CEF_BINARY_PATH_NEW).is_dir() {
        if Path::new(CEF_BINARY_PATH).is_dir() {
            fs::remove_dir_all(CEF_BINARY_PATH)?;
        }
        // mark as fully updated
        fs::rename(CEF_BINARY_PATH_NEW, CEF_BINARY_PATH)?;
        fs::rename(CEF_BINARY_VERSION_PATH_NEW, CEF_BINARY_VERSION_PATH)?;
    }

    Ok(())
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

        fs::create_dir_all(CEF_BINARY_PATH_NEW).unwrap();
        download(CEF_VERSION).await?;

        {
            // mark as half-updated
            let mut f = fs::File::create(CEF_BINARY_VERSION_PATH_NEW).unwrap();
            write!(f, "{}", CEF_VERSION).unwrap();
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

async fn download(version: &str) -> Result<()> {
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

    let maybe_content_length = response.content_length();

    if let Some(content_length) = maybe_content_length {
        print_async(format!(
            "{}Downloading {}{} {}({}{}MB{})",
            color::GOLD,
            //
            color::GREEN,
            "cef-binary",
            color::GOLD,
            //
            color::GREEN,
            (content_length as f32 / 1024f32 / 1024f32).ceil() as u32,
            color::GOLD,
        ))
        .await;
    }

    {
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
                            let out_path = Path::new(CEF_BINARY_PATH_NEW).join(&even_more_trimmed);
                            debug!("{:?} {:?}", path, out_path);

                            let parent = out_path.parent().unwrap();
                            fs::create_dir_all(&parent)?;
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

                // mac needs to extract Chromium Embedded Framework.framework to cef/
                #[cfg(target_os = "macos")]
                {
                    if first_part == "Release" {
                        if let Some(Component::Normal(second_part)) = trimmed_path_components.next()
                        {
                            if second_part == "Chromium Embedded Framework.framework" {
                                let even_more_trimmed: PathBuf = trimmed_path_components.collect();
                                let out_path =
                                    Path::new(CEF_BINARY_PATH_NEW).join(&even_more_trimmed);
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
