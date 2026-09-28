//! Shared backend core (21.02-A). Always refer to this module as `crate::core::…` in `lib.rs` and
//! never write `use core::…` there — that would collide with the built-in `core` crate (P2-37).

pub mod auth;
pub mod db;
pub mod device;
pub mod diag;
pub mod dto;
pub mod error;
pub mod events;
pub mod grants;
pub mod ipc;
pub mod lock;
pub mod poller;
pub mod settings;
pub mod state;
pub mod status;
pub mod terminal;
pub mod tx;
