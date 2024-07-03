mod async_manager;
mod cef_binary_updater;
mod chat_command;
mod github_release_checker;
mod loader;
mod logger;
mod panic;
mod plugin_updater;

use std::{fs, os::raw::c_int, ptr};

use classicube_sys::{
    Chat_Add, Chat_AddOf, IGameComponent, MsgType_MSG_TYPE_CLIENTSTATUS_2, OwnedString,
};
use tracing::*;

extern "C" fn init() {
    panic::install_hook();

    logger::initialize(true, false);

    debug!(
        "Init {}",
        concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"))
    );

    fs::create_dir_all("cef").unwrap();
    cef_binary_updater::prepare().unwrap();
    loader::init();

    chat_command::initialize();

    async_manager::initialize();

    // never update if debug build
    #[cfg(not(debug_assertions))]
    {
        async_manager::spawn(async move {
            loop {
                if let Err(e) = plugin_updater::update_plugins().await {
                    error!("{:#?}", e);
                    print_async(format!(
                        "{}Failed to update CEF: {}{}",
                        classicube_helpers::color::RED,
                        classicube_helpers::color::WHITE,
                        e
                    ))
                    .await;

                    break;
                }

                // check again every 4 hours
                async_manager::sleep(std::time::Duration::from_secs(4 * 60 * 60)).await;
                debug!("auto-checking for updates again");
            }
        });
    }
}

extern "C" fn free() {
    debug!("Free");

    loader::free();

    async_manager::shutdown();
}

extern "C" fn reset() {
    debug!("Reset");

    loader::reset();
}

extern "C" fn on_new_map() {
    debug!("OnNewMap");

    loader::on_new_map();
}

extern "C" fn on_new_map_loaded() {
    debug!("OnNewMapLoaded");

    loader::on_new_map_loaded();
}

#[no_mangle]
pub static Plugin_ApiVersion: c_int = 1;

#[no_mangle]
pub static mut Plugin_Component: IGameComponent = IGameComponent {
    // Called when the game is being loaded.
    Init: Some(init),
    // Called when the component is being freed. (e.g. due to game being closed)
    Free: Some(free),
    // Called to reset the component's state. (e.g. reconnecting to server)
    Reset: Some(reset),
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: Some(on_new_map),
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: Some(on_new_map_loaded),
    // Next component in linked list of components.
    next: ptr::null_mut(),
};

pub fn print<S: Into<String>>(s: S) {
    let mut s = s.into();
    info!("{}", s);

    if s.len() > 255 {
        s.truncate(255);
    }

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub fn status<S: Into<String>>(s: S) {
    let mut s = s.into();
    info!("{}", s);

    if s.len() > 255 {
        s.truncate(255);
    }

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_AddOf(
            owned_string.as_cc_string(),
            MsgType_MSG_TYPE_CLIENTSTATUS_2 as _,
        );
    }
}

pub async fn print_async<S: Into<String> + Send + 'static>(s: S) {
    async_manager::run_on_main_thread(async move {
        print(s);
    })
    .await;
}
