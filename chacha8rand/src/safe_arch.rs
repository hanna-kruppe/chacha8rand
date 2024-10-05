//! Access to `core::arch` intrinsics without spilling `unsafe` everywhere.
//!
//! Unlike the crates.io library of the same name, this module...
//!
//! * Supports runtime feature detection
//! * Only supports the handful of operations needed for implementing chacha8rand
//! * Lacks documentation and coherent naming conventions conventions

#[cfg(target_arch = "x86_64")]
pub mod avx2;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub mod neon;
