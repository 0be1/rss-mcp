//! An MCP server that reads RSS and Atom feeds listed in a TOML file.
//!
//! The crate is split into a library (this file) and a thin binary
//! (`main.rs`) so that integration tests under `tests/` can use the modules.

pub mod config;
pub mod feed;
pub mod server;
