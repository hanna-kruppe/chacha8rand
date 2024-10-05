use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        pub mod avx2;
    } else {
        pub mod avx2 {
            pub fn detect() -> Option<crate::Backend> {
                None
            }
        }
    }
}

cfg_if! {
    if #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little",
    ))] {
        // Limit this to little-endian because the core::arch intrinsics currently don't work on
        // aarch64be (https://github.com/rust-lang/stdarch/issues/1484). Even if they worked, it's a
        // pretty obscure target and difficult to test for (e.g., `cross` doesn't currently support
        // it) so I'm inclined to leave this out until someone champions it.
        pub mod neon;
    } else {
        pub mod neon {
            pub fn detect() -> Option<crate::Backend> {
                None
            }
        }
    }
}

cfg_if! {
    if #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), target_feature ="sse2"))] {
        pub mod sse2;
    } else {
        pub mod sse2 {
            pub fn detect() -> Option<crate::Backend> {
                None
            }
        }
    }
}

pub mod scalar;
pub mod widex4;

// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");
