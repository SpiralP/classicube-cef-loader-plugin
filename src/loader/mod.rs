mod plugin;

use std::cell::Cell;

use classicube_sys::IGameComponent;
use tracing::{debug, error};

use crate::print;

thread_local!(
    static PLUGIN: Cell<Option<*mut IGameComponent>> = const { Cell::new(None) };
);

pub fn init() {
    match plugin::try_init() {
        Ok(plugin_component) => {
            PLUGIN.with(|cell| cell.set(Some(plugin_component)));

            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.Init {
                debug!("Calling Init");
                unsafe {
                    f();
                }
            }
        }

        Err(e) => {
            error!("{:#?}", e);
            print(format!(
                "{}Couldn't load cef plugin: {}{}",
                classicube_helpers::color::RED,
                classicube_helpers::color::WHITE,
                e
            ));
        }
    }
}

pub fn free() {
    PLUGIN.with(|cell| {
        if let Some(plugin_component) = cell.get() {
            cell.set(None);

            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.Free {
                debug!("Calling Free");
                unsafe {
                    f();
                }
            }
        }
    });

    plugin::free();
}

pub fn reset() {
    PLUGIN.with(|cell| {
        if let Some(plugin_component) = cell.get() {
            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.Reset {
                debug!("Calling Reset");
                unsafe {
                    f();
                }
            }
        }
    });
}

pub fn on_new_map() {
    PLUGIN.with(|cell| {
        if let Some(plugin_component) = cell.get() {
            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.OnNewMap {
                debug!("Calling OnNewMap");
                unsafe {
                    f();
                }
            }
        }
    });
}

pub fn on_new_map_loaded() {
    PLUGIN.with(|cell| {
        if let Some(plugin_component) = cell.get() {
            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.OnNewMapLoaded {
                debug!("Calling OnNewMapLoaded");
                unsafe {
                    f();
                }
            }
        }
    });
}
