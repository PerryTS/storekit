//! StoreKit 2 bindings for Perry — closes PerryTS/perry#537.
//!
//! On iOS, tvOS and macOS targets this crate links the Swift StoreKit 2 bridge
//! (see [`apple`]). On every other target it compiles to the same stub
//! behavior as `crate-stub` — no Swift toolchain, no Apple SDK — so the
//! crate builds on any machine with a Rust toolchain (`build.rs` skips
//! the `swiftc` step for non-Apple targets).

#[cfg(any(target_os = "ios", target_os = "macos", target_os = "tvos"))]
mod apple;

#[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "tvos")))]
mod stub;
