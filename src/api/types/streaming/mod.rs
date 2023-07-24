#[derive(serde::Deserialize)]
pub struct StreamResponse<T> {
    pub data: T,
}

mod event_streaming;
pub use event_streaming::*;
