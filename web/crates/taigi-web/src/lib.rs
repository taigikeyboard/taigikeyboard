//! The engine and the dictionaries in a browser tab
//! (`docs/architecture/web-roadmap.md`).
//!
//! The session and the key translation are plain Rust, tested natively. Only
//! `bindings` names the browser: `wasm-bindgen` imports panic off this target
//! and `mmap_host::register` exists only on it, so it is the one module built
//! for wasm alone.

// The plain modules have no caller on a native non-test build, only their tests.
#![cfg_attr(not(any(target_family = "wasm", test)), allow(dead_code))]

#[cfg(target_family = "wasm")]
mod bindings;
mod key;
mod layout;
mod page_settings;
mod reply;
mod session;
mod shortcuts;
