use error_chain::error_chain;
#[allow(unused_imports)]
pub use error_chain::{bail, ensure};

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
        Io(::std::io::Error);
        Reqwest(reqwest::Error);
        Tokio(tokio::task::JoinError);
        Nul(::std::ffi::NulError);
        SerdeJson(serde_json::Error);
    }
}
