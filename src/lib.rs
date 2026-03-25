//! Library modules for the `wax` CLI.
//!
//! The crate is organized around a small pipeline:
//!
//! - [`cli`] defines the command-line surface.
//! - [`app`] wires parsed CLI arguments into runtime settings and output modes.
//! - [`provider`] dispatches provider-specific commands for Bandcamp and SoundCloud.
//! - [`fetch`] and [`cache`] handle HTTP requests and local response caching.
//! - [`parser`], [`soundcloud`], and [`score`] implement provider parsing and ranking.
//! - [`output`] renders results for human and machine consumers.

pub mod app;
pub mod cache;
pub mod cli;
pub mod config;
pub mod error;
pub mod fetch;
pub mod model;
pub mod output;
pub mod parser;
pub mod provider;
pub mod score;
pub mod soundcloud;
