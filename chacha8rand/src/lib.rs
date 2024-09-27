use std::array;

mod backend;
mod guts;
#[cfg(test)]
mod tests;

use arrayref::{array_ref, array_refs};
pub use backend::Backend;

pub struct ChaCha8 {
    seed: [u32; 8],
    i: usize,
    buf: [u32; 256],
    backend: Backend,
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
        let mut this = Self {
            seed: seed.0,
            i: 0,
            buf: [0; 256],
            backend,
        };
        backend.refill(&this.seed, &mut this.buf);
        this
    }

    pub fn next_u32(&mut self) -> u32 {
        let (output_buf, next_seed) = array_refs![&self.buf, 248, 8];
        // There doesn't seem to be a reliable, stable way to convince the compiler that this branch
        // is unlikely. For example, #[cold] on Backend::refill is ignored at the time of this
        // writing. Out of the various ways I've tried writing this function, this one seems to
        // generate the least bad assembly when compiled in isolation. (Of course, in practice we
        // want it to be inlined.)
        if self.i >= output_buf.len() {
            self.seed = *next_seed;
            self.backend.refill(&self.seed, &mut self.buf);
            self.i = 0;
        }
        // Can't use `output_buf` here because the refill branch clobbered that borrow, but it's
        // okay because we ensured `i < output_buf.len() < self.buf.len()`.
        let result = self.buf[self.i];
        self.i += 1;
        result
    }
}
