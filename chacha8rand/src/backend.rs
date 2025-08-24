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

    // The neon backend is limited to little-endian because aarch64be NEON has historically been
    // broken (https://github.com/rust-lang/stdarch/issues/1484). That's allegedly fixed now, but
    // it's hard to test for these targets (e.g., `cross` doesn't currently support it) so let's err
    // on the side of correctness for now. The scalar code is tested on big endian (via s390x).
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

fn init_state<T: Copy>(ctr: T, key: &[u32; 8], splat: impl Fn(u32) -> T) -> [T; 16] {
    const C0: u32 = u32::from_le_bytes(*b"expa");
    const C1: u32 = u32::from_le_bytes(*b"nd 3");
    const C2: u32 = u32::from_le_bytes(*b"2-by");
    const C3: u32 = u32::from_le_bytes(*b"te k");

    let [k0, k1, k2, k3, k4, k5, k6, k7] = *key;
    #[rustfmt::skip]
    let x = [
        splat(C0), splat(C1), splat(C2), splat(C3),
        splat(k0), splat(k1), splat(k2), splat(k3),
        splat(k4), splat(k5), splat(k6), splat(k7),
        ctr,       splat(0),  splat(0),  splat(0),
    ];
    x
}

#[inline]
fn eight_rounds<T: Copy>(x: &mut [T; 16], qr: impl Fn([T; 4]) -> [T; 4]) {
    const ROUNDS: u32 = 8;
    for _ in (0..ROUNDS).step_by(2) {
        // Odd round: columns
        [x[0], x[4], x[8], x[12]] = qr([x[0], x[4], x[8], x[12]]);
        [x[1], x[5], x[9], x[13]] = qr([x[1], x[5], x[9], x[13]]);
        [x[2], x[6], x[10], x[14]] = qr([x[2], x[6], x[10], x[14]]);
        [x[3], x[7], x[11], x[15]] = qr([x[3], x[7], x[11], x[15]]);
        // Even round: diagonals
        [x[0], x[5], x[10], x[15]] = qr([x[0], x[5], x[10], x[15]]);
        [x[1], x[6], x[11], x[12]] = qr([x[1], x[6], x[11], x[12]]);
        [x[2], x[7], x[8], x[13]] = qr([x[2], x[7], x[8], x[13]]);
        [x[3], x[4], x[9], x[14]] = qr([x[3], x[4], x[9], x[14]]);
    }
}
