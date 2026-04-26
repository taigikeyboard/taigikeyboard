#![allow(clippy::all, clippy::pedantic)]

pub mod engine {
    include!(concat!(env!("OUT_DIR"), "/taigi.engine.rs"));
}

pub use engine::*;
