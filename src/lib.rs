mod async_manager;
mod cef_binary_updater;
mod error;
mod github_release_checker;
mod loader;
mod logger;
mod plugin_updater;

use crate::plugin_updater::update_plugins;
use classicube_sys::{
    Chat_Add, Chat_AddOf, IGameComponent, MsgType_MSG_TYPE_CLIENTSTATUS_2, OwnedString,
};
use log::{debug, info};
use std::{cell::RefCell, os::raw::c_int, ptr};

// queue callback kinds while we are still updating, then call them all after we init()
thread_local!(
    static CALLBACK_QUEUE: RefCell<Option<Vec<CallbackKind>>> = RefCell::new(Some(Vec::new()));
);

enum CallbackKind {
    Free,
    Reset,
    OnNewMap,
    OnNewMapLoaded,
}

fn queue_callback(kind: CallbackKind) {
    CALLBACK_QUEUE.with(|cell| {
        let maybe_queue = &mut *cell.borrow_mut();
        if let Some(queue) = maybe_queue {
            // not yet init'd, queue for right after we init
            queue.push(kind);
        } else {
            // already init'd
            run_callback(kind);
        }
    });
}

fn run_callback(kind: CallbackKind) {
    match kind {
        CallbackKind::Free => loader::free(),
        CallbackKind::Reset => loader::reset(),
        CallbackKind::OnNewMap => loader::on_new_map(),
        CallbackKind::OnNewMapLoaded => loader::on_new_map_loaded(),
    }
}

extern "C" fn init() {
    color_backtrace::install_with_settings(
        color_backtrace::Settings::new().verbosity(color_backtrace::Verbosity::Full),
    );

    logger::initialize(true, false);

    debug!("Init");

    async_manager::initialize();

    async_manager::spawn(async move {
        #[cfg(not(debug_assertions))]
        let result = update_plugins().await;

        // never update if debug build
        #[cfg(debug_assertions)]
        let result = if false {
            update_plugins().await
        } else {
            Ok(())
        };

        if let Err(e) = result {
            print_async(format!(
                "{}Failed to update: {}{}",
                classicube_helpers::color::RED,
                classicube_helpers::color::WHITE,
                e
            ))
            .await;
        }

        async_manager::spawn_on_main_thread(async {
            loader::init();

            async_manager::mark_for_shutdown();

            CALLBACK_QUEUE.with(|cell| {
                let maybe_queue = &mut *cell.borrow_mut();

                for kind in maybe_queue.take().unwrap().drain(..) {
                    run_callback(kind);
                }
            });
        });
    });
}

extern "C" fn free() {
    debug!("Free");

    queue_callback(CallbackKind::Free);

    async_manager::shutdown();
}

extern "C" fn reset() {
    debug!("Reset");

    queue_callback(CallbackKind::Reset);

    async_manager::check_should_shutdown();
}

extern "C" fn on_new_map() {
    debug!("OnNewMap");

    queue_callback(CallbackKind::OnNewMap);

    async_manager::check_should_shutdown();
}

extern "C" fn on_new_map_loaded() {
    debug!("OnNewMapLoaded");

    queue_callback(CallbackKind::OnNewMapLoaded);

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
