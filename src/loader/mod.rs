mod plugin;

use std::cell::Cell;

use classicube_sys::IGameComponent;
use tracing::{debug, error};

use crate::print;

thread_local!(
    static PLUGIN: Cell<Option<*mut IGameComponent>> = const { Cell::new(None) };
);

pub fn init() {
    if PLUGIN.with(|cell| cell.get().is_some()) {
        debug!("inner plugin already loaded; skipping re-init");
        return;
    }

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
    // Forward `IGameComponent::Free` to the inner plugin so it can run
    // its shutdown (notably `entity_manager.shutdown()`, which drops
    // `OwnedGfxTexture`/`OwnedGfxVertexBuffer`). Without those drops the
    // D3D9 device retains outstanding refs and d3d9.dll's
    // DLL_PROCESS_DETACH hangs at process exit.
    //
    // We deliberately do NOT `dlclose` the inner plugin here: its chat
    // command is pinned in ClassiCube's `cmds_head` list with no
    // unregister API, so unloading the DLL would leave dangling function
    // pointers. The inner plugin's commit 30d14e15 made its shutdown
    // safe to call without a subsequent `dlclose`.
    //
    // `cell.take()` makes this idempotent at the loader level — a second
    // Free is a no-op, so we don't trip the inner plugin's
    // `plugin.take().unwrap()` panic on double-free.
    PLUGIN.with(|cell| {
        if let Some(plugin_component) = cell.take() {
            let plugin_component = unsafe { &mut *plugin_component };

            if let Some(f) = plugin_component.Free {
                debug!("Calling Free");
                unsafe {
                    f();
                }
            }
        }
    });
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
