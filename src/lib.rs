mod loader;
mod logger;
mod panic;
mod updater;

use std::{cell::Cell, fs, os::raw::c_int, ptr};

use classicube_helpers::async_manager;
use classicube_sys::{
    Chat_Add, Chat_AddOf, IGameComponent, MsgType_MSG_TYPE_CLIENTSTATUS_2, OwnedString,
};
use tracing::*;

thread_local!(
    static LOADED: Cell<bool> = const { Cell::new(false) };
);

extern "C" fn init() {
    panic::install_hook();

    logger::initialize(true, false);

    debug!(
        "Init {}",
        concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"))
    );

    fs::create_dir_all("cef").unwrap();

    async_manager::initialize();
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

    if LOADED.get() {
        loader::on_new_map_loaded();
    } else {
        LOADED.set(true);

        async_manager::spawn(async move {
            // don't update if debug build
            if cfg!(not(debug_assertions)) {
                if let Err(e) = updater::update_plugins().await {
                    error!("{:#?}", e);
                    print_async(format!(
                        "{}Failed to update CEF: {}{e}",
                        classicube_helpers::color::RED,
                        classicube_helpers::color::WHITE
                    ))
                    .await;
                }
            }

            async_manager::spawn_on_main_thread(async move {
                loader::init();
                loader::on_new_map();
                loader::on_new_map_loaded();
            });
        });
    }
}

#[allow(non_upper_case_globals)]
#[no_mangle]
pub static Plugin_ApiVersion: c_int = 1;

#[allow(non_upper_case_globals)]
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
