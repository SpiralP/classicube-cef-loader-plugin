mod async_manager;
mod cef_binary_updater;
mod error;
mod github_release_checker;
mod loader;
mod plugin_updater;

use crate::{async_manager::AsyncManager, plugin_updater::update_plugins};
use classicube_sys::IGameComponent;
use std::{cell::RefCell, os::raw::c_int, ptr};

thread_local!(
    static ASYNC_MANAGER: RefCell<AsyncManager> = RefCell::new(AsyncManager::new());
);

extern "C" fn init() {
    println!("Init");

    ASYNC_MANAGER.with(|cell| {
        let async_manager = &mut *cell.borrow_mut();
        async_manager.initialize();
    });

    update_plugins();

    loader::init();
}

extern "C" fn on_new_map_loaded() {
    loader::on_new_map_loaded();
}

extern "C" fn free() {
    println!("Free");

    loader::free();

    ASYNC_MANAGER.with(|cell| {
        let async_manager = &mut *cell.borrow_mut();
        async_manager.shutdown();
    });
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
    Reset: None,
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: None,
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: Some(on_new_map_loaded),
    // Next component in linked list of components.
    next: ptr::null_mut(),
};

pub fn print<S: Into<String>>(s: S) {
    use classicube_sys::{Chat_Add, OwnedString};

    let s = s.into();
    println!("{}", s);

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub async fn print_async<S: Into<String> + Send + 'static>(s: S) {
    AsyncManager::run_on_main_thread(async move {
        print(s);
    })
    .await;
}