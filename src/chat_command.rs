use crate::print;
use classicube_sys::OwnedChatCommand;
use log::*;
use std::{cell::RefCell, os::raw::c_int, slice};

pub fn initialize() {
    thread_local!(
        static CHAT_COMMAND: RefCell<OwnedChatCommand> = RefCell::new(OwnedChatCommand::new(
            "CefLoader",
            c_chat_command_callback,
            false,
            vec!["cef-loader"],
        ));
    );

    CHAT_COMMAND.with(|cell| {
        cell.borrow_mut().register();
    });
}

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    handle_command(args);
}

fn handle_command(args: Vec<String>) {
    debug!("chat command: {:?}", args);

    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
    match args.as_slice() {
        ["update"] | ["check"] => {
            crate::check_updates();
        }

        _ => {
            print("/client CefLoader [update]");
        }
    }
}
