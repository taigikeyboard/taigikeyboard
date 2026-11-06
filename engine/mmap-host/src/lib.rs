//! `mmap-host` — read-only byte source for the dictionary files.
//!
//! Native targets mmap the file. That backend owns the only NEW
//! `unsafe_code = "allow"` carve-out introduced by the v3.5.6 lexicon slice:
//! the `unsafe` block lives in `MmapHandle::open_readonly` and is gated by a
//! single SAFETY comment per `docs/contributing/rust-ffi-safety.md` §3.
//!
//! `wasm32` has no file system to map, so that backend answers from a registry
//! of bytes the host put in memory (`docs/architecture/web-roadmap.md`).
//!
//! All consumers (currently `engine/lexicon`) take a `&[u8]` whose lifetime
//! is tied to the `&MmapHandle` borrow, so neither backend leaks into them.

#[derive(thiserror::Error, Debug)]
pub enum MmapError {
    #[error("open `{path}`: {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("mmap `{path}`: {source}")]
    Map {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(not(target_family = "wasm"))]
pub use native::MmapHandle;

#[cfg(target_family = "wasm")]
mod registry;
#[cfg(target_family = "wasm")]
pub use registry::{register, MmapHandle};
