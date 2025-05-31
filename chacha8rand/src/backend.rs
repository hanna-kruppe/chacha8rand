// Conceptually this belongs here, not in a submodule, but we need the extra module to have a
// private field and enforce the use of the constructor. Unsafe fields would simplify this.
pub(crate) use details::Backend;
mod details;

mod scalar;

pub(crate) fn scalar() -> Backend {
    Backend::new(scalar::fill_buf)
}

macro_rules! arch_backends {
    ($(#[cfg($cond:meta)] mod $name:ident;)+) => {
        $(
            #[cfg($cond)]
            pub(crate) mod $name;

            #[cfg(not($cond))]
            pub(crate) mod $name {
                pub(crate) fn detect() -> Option<crate::Backend> {
                    None
                }
            }
        )+
    };
}

arch_backends! {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    mod avx2;

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    mod sse2;

    // The neon backend is limited to little-endian because the core::arch intrinsics currently
    // don't work on aarch64be (https://github.com/rust-lang/stdarch/issues/1484). Even if they
    // worked, it's a pretty obscure target and difficult to test for (e.g., `cross` doesn't
    // currently support it) so I'm inclined to leave this out until someone champions it.
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    mod neon;

    // Wasm validation doesn't play nice with runtime detection, so we do static detection only.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    mod simd128;
}

pub(crate) fn detect_best() -> Backend {
    // On x86, we prefer AVX2 over SSE2 when both are available. The other SIMD backends aren't
    // really ordered by preference because they're for mutually exclusive target platforms, but
    // it's less of a mess to chain them like this than to replicate the `cfg` soup. We only use
    // the scalar backend if none of the SIMD backends are available.
    avx2::detect()
        .or_else(sse2::detect)
        .or_else(neon::detect)
        .or_else(simd128::detect)
        .unwrap_or_else(scalar)
}
