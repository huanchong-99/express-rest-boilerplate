//! Route definitions matching the Express.js v1 route structure.
//!
//! This module re-exports route group functions used by `lib.rs` to build
//! the main router. The actual handler implementations live in
//! `src/handlers/`.

pub mod auth;
pub mod user;