//! Cliente HTTP dos serviços do Anitsu, sobre o reqwest.
//!
//! Tokens nunca saem do núcleo: a interface chama [`request`] e o [`AnitsuHttp`]
//! injeta a sessão (cookie ou bearer) conforme o serviço.

mod api;
mod client;
mod error;

pub use api::{RequestArgs, request};
pub use client::{AnitsuApis, AnitsuHttp, Service};
pub use error::{HttpError, HttpResult};
