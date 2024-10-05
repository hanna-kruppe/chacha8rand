#![deny(unsafe_op_in_unsafe_fn)]
use std::array;

mod backend;
mod guts;
#[cfg(feature = "rand_core_0_6")]
pub mod rand_core_0_6;
mod safe_arch;
#[cfg(test)]
mod tests;

use arrayref::array_ref;

pub use backend::Backend;

// This is repr(C) because rustc's heuristic for minimizing padding puts the buffer first, which
// doesn't actually reduce padding compared to this layout and increases the offsets of all fields
// to slightly more than 1000 bytes. That's still within the range of immediate offsets for
// loads/stores in most instruction sets, but takes slightly more space to encode in Webassembly.
#[repr(C)]
pub struct ChaCha8 {
    backend: Backend,
    i: usize,
    seed: [u32; 8],
    buf: Buffer,
}

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

fn backend_detect() -> Backend {
    if let Some(avx2) = Backend::avx2() {
        return avx2;
    }
    if cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64") {
        // These targets always have 128 bit SIMD available
        return Backend::widex4();
    }
    if cfg!(target_arch = "wasm32") && cfg!(target_feature = "simd128") {
        // No dynamic feature detection on wasm.
        return Backend::widex4();
    }
    if cfg!(target_arch = "x86") && cfg!(target_feature = "sse2") {
        // The case for the x4 impl is less obvious for 32-bit x86 SIMD because there we only have
        // eight XMM registers, but it's probably no worse than the scalar implementation. TODO:
        // benchmark it.
        return Backend::widex4();
    }
    // Fallback if we don't know for sure that we have SIMD:
    Backend::scalar()
}

impl ChaCha8 {
    pub fn new(seed: Seed) -> Self {
        Self::with_backend(seed, backend_detect())
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
