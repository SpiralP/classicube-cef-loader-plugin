mod async_manager;
mod cef_binary_updater;
mod chat_command;
mod error;
mod github_release_checker;
mod loader;
mod logger;
mod plugin_updater;

use classicube_sys::{
    Chat_Add, Chat_AddOf, IGameComponent, MsgType_MSG_TYPE_CLIENTSTATUS_2, OwnedString,
};
use log::{debug, info};
use std::{fs, os::raw::c_int, ptr};

extern "C" fn init() {
    color_backtrace::install_with_settings(
        color_backtrace::Settings::new()
            .verbosity(color_backtrace::Verbosity::Full)
            .message("CEF Loader crashed!!"),
    );

    logger::initialize(true, false);

    debug!("Init");

    fs::create_dir_all("cef").unwrap();
    cef_binary_updater::prepare().unwrap();
    loader::init();

    // never update if debug build
    #[cfg(not(debug_assertions))]
    check_updates();

    chat_command::initialize();
}

pub fn check_updates() {
    async_manager::initialize();
    async_manager::spawn(async move {
        if let Err(e) = plugin_updater::update_plugins().await {
            print_async(format!(
                "{}Failed to update: {}{}",
                classicube_helpers::color::RED,
                classicube_helpers::color::WHITE,
                e
            ))
            .await;
        }

        async_manager::spawn_on_main_thread(async {
            async_manager::mark_for_shutdown();
        });
    });
}

extern "C" fn free() {
    debug!("Free");

    loader::free();

    async_manager::shutdown();
}

extern "C" fn reset() {
    debug!("Reset");

    loader::reset();

    async_manager::check_should_shutdown();
}

extern "C" fn on_new_map() {
    debug!("OnNewMap");

    loader::on_new_map();

    async_manager::check_should_shutdown();
}

extern "C" fn on_new_map_loaded() {
    debug!("OnNewMapLoaded");

    loader::on_new_map_loaded();

    async_manager::check_should_shutdown();
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
