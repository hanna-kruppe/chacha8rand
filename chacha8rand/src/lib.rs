#![forbid(unsafe_op_in_unsafe_fn)]
#![no_std]
use core::array;

// On x86 we use runtime feature detection, which is currently only supported in std or with
// third-party libraries. We could use one of those libraries to be no_std even on x86, but I don't
// have a use case for that so let's just pull in std while we wait for runtime feature detection in
// core to be implemented and stabilized.
#[cfg(target_arch = "x86_64")]
extern crate std;

mod backend;
mod guts;
#[cfg(feature = "rand_core_0_6")]
pub mod rand_core_0_6;
mod safe_arch;
#[cfg(test)]
mod tests;

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

    pub fn next_u32(&mut self) -> u32 {
        // There doesn't seem to be a reliable, stable way to convince the compiler that this branch
        // is unlikely. For example, #[cold] on Backend::refill is ignored at the time of this
        // writing. Out of the various ways I've tried writing this function, this one seems to
        // generate the least bad assembly when compiled in isolation. (Of course, in practice we
        // want it to be inlined.)
        if self.i >= self.buf.output().len() {
            self.seed = *self.buf.new_key();
            self.backend.refill(&self.seed, &mut self.buf);
            self.i = 0;
        }
        let result = self.buf.output()[self.i];
        self.i += 1;
        result
    }
}

// This impl block is here, not in the `backend` mod, to minimize that code that has access to
// `Backend`'s private fields.
impl Backend {
    pub fn detect_best() -> Self {
        if let Some(avx2) = Backend::x86_avx2() {
            // This is the best choice on x86 when it exists.
            return avx2;
        }
        if let Some(neon) = Backend::aarch64_neon() {
            return neon;
        }
        if cfg!(target_arch = "x86_64") {
            // These targets always have 128 bit SIMD available.
            return Backend::widex4();
        }
        if cfg!(target_arch = "wasm32") && cfg!(target_feature = "simd128") {
            // No dynamic feature detection on wasm.
            return Backend::widex4();
        }
        if cfg!(target_arch = "x86") && cfg!(target_feature = "sse2") {
            // The case for the x4 impl is less obvious for 32-bit x86 SIMD because there we only
            // have eight XMM registers, but it's probably no worse than the scalar implementation.
            // TODO: benchmark it.
            return Backend::widex4();
        }
        // Fallback if we don't know for sure that we have SIMD:
        Backend::scalar()
    }

    pub fn scalar() -> Backend {
        Self::new(guts::scalar::fill_buf)
    }

    pub fn widex4() -> Backend {
        Self::new(guts::widex4::fill_buf)
    }

    pub fn x86_avx2() -> Option<Self> {
        guts::avx2::detect()
    }

    pub fn aarch64_neon() -> Option<Self> {
        guts::neon::detect()
    }
}
