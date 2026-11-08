#!/bin/sh
# `zig cc` as the wasm32 C compiler for SQLite's C (`sqlite-wasm-rs`), so the
# build needs no LLVM clang beyond the zig mise.toml pins: Apple's clang has no
# WebAssembly backend. cc-rs passes Rust's `--target=wasm32-unknown-unknown`,
# whose `unknown` OS zig refuses; zig names the same target
# `wasm32-freestanding`. Every other argument passes through.
for arg in "$@"; do
  shift
  case $arg in
    --target=wasm32-unknown-unknown) set -- "$@" -target wasm32-freestanding ;;
    *) set -- "$@" "$arg" ;;
  esac
done
exec zig cc "$@"
