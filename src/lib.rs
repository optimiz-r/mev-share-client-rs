#![feature(let_chains)]
#![feature(async_closure)]
#![feature(async_fn_in_trait)]
#![feature(lazy_cell)]
#![feature(concat_idents)]

mod api;
mod client;
mod error;
pub mod prelude;
mod provider;

pub use api::networks::Network;
pub use api::types::Buildable;
pub use error::{Error, Result, *};
pub use prelude::*;
