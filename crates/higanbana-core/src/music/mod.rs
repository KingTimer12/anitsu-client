//! Player de músicas: lê faixas da nuvem por Range, decodifica e toca pelo Rust.

pub mod album;
mod api;
pub mod decode;
pub mod engine;
pub mod gain;
pub mod image;
pub mod media_keys;
pub mod output;
pub mod queue;
pub mod source;
pub mod store;

pub use api::{Music, TrackMeta, parse_cmd, prefix_end};
