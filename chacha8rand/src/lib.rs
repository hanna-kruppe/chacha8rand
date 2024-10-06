#![forbid(unsafe_op_in_unsafe_fn)]
#![no_std]
use core::array;

mod backend;
#[cfg(feature = "rand_core_0_6")]
pub mod rand_core_0_6;
mod scalar;
#[cfg(test)]
mod tests;
mod widex4;

use arrayref::array_ref;

pub use backend::Backend;

// Note: rustc's field reordering heuristc puts the buffer before the other fields because it has
// the highest alignment. There are other layouts that also minimize padding, but the one rustc
// picks happen to generate slightly better code for `next_u32` on some targets (e.g., on aarch64,
// it avoids computing the address of the buffer before checking if it needs to be refilled).
pub struct ChaCha8 {
    backend: Backend,
    i: usize,
    seed: [u32; 8],
    buf: Buffer,
}

// None of the backends currently require this alignment for soundness, but SIMD memory accesses
// that cross 32- or 64-byte boundaries are slightly slower on a bunch of CPUs, so higher alignment
// is occasionally useful. Since we don't do 512-bit SIMD, 32-byte alignment is sufficient.
#[repr(align(32))]
pub struct Buffer {
    pub words: [u32; 256],
}

impl Buffer {
    fn output(&self) -> &[u32; 248] {
        array_ref![&self.words, 0, 248]
    }

    fn new_key(&self) -> &[u32; 8] {
        array_ref![&self.words, 248, 8]
    }
}

pub struct Seed([u32; 8]);

impl From<[u8; 32]> for Seed {
    fn from(bytes: [u8; 32]) -> Self {
        Self(array::from_fn(|i| {
            u32::from_le_bytes(*array_ref![bytes, 4 * i, 4])
        }))
    }
}

impl From<&[u8; 32]> for Seed {
    fn from(bytes: &[u8; 32]) -> Self {
        Self::from(*bytes)
    }
}

impl ChaCha8 {
    pub fn new(seed: Seed) -> Self {
        Self::with_backend(seed, Backend::detect_best())
    }

    pub fn with_backend(seed: Seed, backend: Backend) -> Self {
        let buf = Buffer { words: [0; 256] };
        let mut this = Self {
            seed: seed.0,
            i: 0,
            buf,
            backend,
        };
        backend.refill(&this.seed, &mut this.buf);
        this
    }

    #[inline]
    fn refill(&mut self) {
        self.seed = *self.buf.new_key();
        self.backend.refill(&self.seed, &mut self.buf);
        self.i = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        // There doesn't seem to be a reliable, stable way to convince the compiler that this branch
        // is unlikely. For example, #[cold] on Backend::refill is ignored at the time of this
        // writing. Out of the various ways I've tried writing this function, this one seems to
        // generate the least bad assembly when compiled in isolation. (Of course, in practice we
        // want it to be inlined.)
        if self.i >= self.buf.output().len() {
            self.refill();
        }
        let result = self.buf.output()[self.i];
        self.i += 1;
        result
    }

    pub fn next_u64(&mut self) -> u64 {
        let lo_half = u64::from(self.next_u32());
        let hi_half = u64::from(self.next_u32());
        (hi_half << 32) | lo_half
    }
}

macro_rules! arch_backends {
    (#[cfg($($cond:meta)*)] mod $name:ident; $($rest:tt)*) => {
        #[cfg($($cond)*)]
        mod $name {
            mod safe_arch;
            mod backend;
            pub(crate) use backend::detect;
        }

        #[cfg(not($($cond)*))]
        mod $name {
            pub fn detect() -> Option<crate::Backend> {
                None
            }
        }

        arch_backends! { $($rest)* }
    };

    () => {};
}

arch_backends! {
    // This backend uses dynamic feature detection, so it's only gated on `target_arch` and not on
    // `target_feature = "avx2"`.
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    mod avx2;

    // For SSE2 we don't bother with dynamic feature detection. x86_64 basically always has it, it's
    // also very commonly enabled on 32-bit targets, and when it isn't, we still have a very high
    // chance that AVX2 is available at runtime.
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), target_feature = "sse2"))]
    mod sse2;

    // The neon backend is limited to little-endian because the core::arch intrinsics currently
    // don't work on aarch64be (https://github.com/rust-lang/stdarch/issues/1484). Even if they
    // worked, it's a pretty obscure target and difficult to test for (e.g., `cross` doesn't
    // currently support it) so I'm inclined to leave this out until someone champions it.
    #[cfg(all(target_arch = "aarch64", target_feature = "neon", target_endian = "little"))]
    mod neon;
}

// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");

// This impl block is here, not in the `backend` mod, to minimize that code that has access to
// `Backend`'s private fields.
impl Backend {
    pub fn detect_best() -> Self {
        // On x86, we prefer AVX2 where available, otherwise we'll almost always have SSE2 without
        // runtime detection.
        if let Some(avx2) = Backend::x86_avx2() {
            return avx2;
        }
        if let Some(sse2) = Backend::x86_sse2() {
            return sse2;
        }

        if let Some(neon) = Backend::aarch64_neon() {
            return neon;
        }

        if cfg!(target_arch = "wasm32") && cfg!(target_feature = "simd128") {
            // No dynamic feature detection on wasm.
            return Backend::widex4();
        }
        // Fallback if we don't know for sure that we have SIMD:
        Backend::scalar()
    }

    pub fn scalar() -> Backend {
        Self::new(scalar::fill_buf)
    }

    pub fn widex4() -> Backend {
        Self::new(widex4::fill_buf)
    }

    pub fn x86_avx2() -> Option<Self> {
        avx2::detect()
    }

    pub fn x86_sse2() -> Option<Self> {
        sse2::detect()
    }

    pub fn aarch64_neon() -> Option<Self> {
        neon::detect()
    }
}
