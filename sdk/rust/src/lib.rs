//! Thin Rust SDK re-exports for plugin authors.

#![forbid(unsafe_code)]

pub use kelicad_plugin::{
    Manifest, NativeGuest, Permissions, PluginError, PluginHost, ScriptGuest,
};
