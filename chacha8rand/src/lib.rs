#![forbid(unsafe_op_in_unsafe_fn)]
#![no_std]
use core::{array, cmp, error::Error, fmt};

use arrayref::array_ref;

mod backend;
#[cfg(feature = "rand_core_0_6")]
mod rand_core_0_6;
mod scalar;
#[cfg(test)]
mod tests;

pub use backend::Backend;

const BUF_TOTAL_LEN: usize = 1024;
const BUF_OUTPUT_LEN: usize = BUF_TOTAL_LEN - 32;

// Note: rustc's field reordering heuristc puts the buffer before the other fields because it has
// the highest alignment. There are other layouts that also minimize padding, but the one rustc
// picks happen to generate slightly better code for `next_u32` on some targets (e.g., on aarch64,
// it avoids computing the address of the buffer before checking if it needs to be refilled).
#[derive(Clone)]
pub struct ChaCha8Rand {
    backend: Backend,
    seed: [u32; 8],
    /// Position in `buf.output()` of the next byte to produce as output. Should be equal to
    /// [`BUF_OUTPUT_LEN`] if all the output was already consumed. Larger values are not canonical
    /// and *shouldn't* occur, but we never rely on this for safety and most other code also tries
    /// to handle larger values gracefully.
    bytes_consumed: usize,
    buf: Buffer,
}

#[derive(Clone, Copy)]
pub struct ChaCha8State {
    pub seed: [u32; 8],
    pub bytes_consumed: u32,
}

impl fmt::Debug for ChaCha8State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChaCha8State {}")
    }
}

// None of the backends currently require this alignment for soundness, but SIMD memory accesses
// that cross 32- or 64-byte boundaries are slightly slower on a bunch of CPUs, so higher alignment
// is occasionally useful. Since we don't do 512-bit SIMD, 32-byte alignment is sufficient.
#[repr(align(32))]
#[derive(Clone)]
struct Buffer {
    bytes: [u8; BUF_TOTAL_LEN],
}

impl Buffer {
    fn output(&self) -> &[u8; BUF_OUTPUT_LEN] {
        array_ref![&self.bytes, 0, BUF_OUTPUT_LEN]
    }

    fn new_key(&self) -> &[u8; 32] {
        array_ref![&self.bytes, BUF_OUTPUT_LEN, 32]
    }
}

pub struct Seed(pub [u32; 8]);

impl From<[u32; 8]> for Seed {
    fn from(words: [u32; 8]) -> Self {
        Self(words)
    }
}

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

impl From<&[u32; 8]> for Seed {
    fn from(words: &[u32; 8]) -> Self {
        Self(*words)
    }
}

pub struct RestoreStateError {
    _private: (),
}

impl fmt::Debug for RestoreStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RestoreStateError")
    }
}

impl fmt::Display for RestoreStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("could not restore ChaCha8State")
    }
}

impl Error for RestoreStateError {}

impl ChaCha8Rand {
    pub fn new(seed: impl Into<Seed>) -> Self {
        fn inner(seed: &Seed) -> ChaCha8Rand {
            ChaCha8Rand::with_backend_mono(seed, Backend::detect_best())
        }
        inner(&seed.into())
    }

    pub fn with_backend(seed: impl Into<Seed>, backend: Backend) -> Self {
        Self::with_backend_mono(&seed.into(), backend)
    }

    fn with_backend_mono(seed: &Seed, backend: Backend) -> ChaCha8Rand {
        let mut this = ChaCha8Rand {
            seed: [0; 8],
            bytes_consumed: 0,
            buf: Buffer { bytes: [0; 1024] },
            backend,
        };
        this.set_seed_mono(seed);
        this
    }

    pub fn set_seed(&mut self, seed: impl Into<Seed>) {
        self.set_seed_mono(&seed.into())
    }

    fn set_seed_mono(self: &mut ChaCha8Rand, seed: &Seed) {
        // Fill the buffer immediately because we want the next bytes of output to come directly
        // from the new seed, not from the old seed or from the seed *after* `seed`.
        self.backend.refill(&seed.0, &mut self.buf);
        self.seed = seed.0;
        self.bytes_consumed = 0;
    }

    pub fn clone_state(&self) -> ChaCha8State {
        // The cast to u32 can't truncate because we never set the field to anything larger than the
        // size of the output buffer. But if that does happen, restoring from the resulting state
        // could behave incorrectly. That code path is also careful about it but defense in depth
        // can't hurt, so let's saturate here.
        debug_assert!(self.bytes_consumed <= BUF_OUTPUT_LEN);
        let bytes_consumed = cmp::min(self.bytes_consumed, BUF_OUTPUT_LEN) as u32;
        ChaCha8State {
            seed: self.seed,
            bytes_consumed,
        }
    }

    pub fn try_restore_state(&mut self, state: &ChaCha8State) -> Result<(), RestoreStateError> {
        // We never produce `bytes_consumed` values larger than the output buffer's size. The u32 ->
        // usize conversion can only fail on 16-bit targets, which are not exactly the target
        // audience for this crate, but we can easily handle that while we're here.
        let bytes_consumed = usize::try_from(state.bytes_consumed).unwrap_or(usize::MAX);
        if bytes_consumed > BUF_OUTPUT_LEN {
            return Err(RestoreStateError { _private: () });
        }

        // We can just use `set_seed` to fill the buffer and then skip the parts of that chunk that
        // were marked as already consumed by adjusting our position in the refilled buffer.
        self.set_seed(state.seed);
        self.bytes_consumed = bytes_consumed;
        Ok(())
    }

    #[inline]
    fn refill(&mut self) {
        self.seed = Seed::from(self.buf.new_key()).0;
        self.backend.refill(&self.seed, &mut self.buf);
        self.bytes_consumed = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        const N: usize = size_of::<u32>();

        // There doesn't seem to be a reliable, stable way to convince the compiler that this branch
        // is unlikely. For example, #[cold] on Backend::refill is ignored at the time of this
        // writing. Out of the various ways I've tried writing this function, this one seems to
        // generate the least bad assembly when compiled in isolation. (Of course, in practice we
        // want it to be inlined.)
        if self.bytes_consumed > BUF_OUTPUT_LEN - N {
            self.refill();
        }
        let bytes = *array_ref![self.buf.output(), self.bytes_consumed, N];
        self.bytes_consumed += N;
        u32::from_le_bytes(bytes)
    }

    pub fn next_u64(&mut self) -> u64 {
        const N: usize = size_of::<u64>();
        // Same code as for u32. Making this code generic over `N` is more trouble than it's worth.
        if self.bytes_consumed > BUF_OUTPUT_LEN - N {
            self.refill();
        }
        let bytes = *array_ref![self.buf.output(), self.bytes_consumed, N];
        self.bytes_consumed += N;
        u64::from_le_bytes(bytes)
    }

    pub fn read_bytes(&mut self, dest: &mut [u8]) {
        let mut total_bytes_read = 0;
        while total_bytes_read < dest.len() {
            let dest_remainder = &mut dest[total_bytes_read..];
            if self.bytes_consumed >= self.buf.output().len() {
                self.refill();
            }
            let src = &self.buf.output()[self.bytes_consumed..];
            let read_now = cmp::min(src.len(), dest_remainder.len());

            dest_remainder[..read_now].copy_from_slice(&src[..read_now]);

            total_bytes_read += read_now;
            self.bytes_consumed += read_now;
            debug_assert!(self.bytes_consumed <= self.buf.output().len());
        }
        debug_assert!(total_bytes_read == dest.len());
    }
}

impl fmt::Debug for ChaCha8Rand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChaCha8 {}")
    }
}

macro_rules! arch_backends {
    ($(#[cfg($cond:meta)] mod $name:ident;)+) => {
        $(
            #[cfg($cond)]
            mod $name {
                mod safe_arch;
                mod backend;
                pub(crate) use backend::detect;
            }

            #[cfg(not($cond))]
            mod $name {
                pub fn detect() -> Option<crate::Backend> {
                    None
                }
            }
        )+
    };
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

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    mod simd128;
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

        if let Some(simd128) = Backend::wasm32_simd128() {
            return simd128;
        }

        Backend::scalar()
    }

    pub fn scalar() -> Backend {
        Self::new(scalar::fill_buf)
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

    pub fn wasm32_simd128() -> Option<Self> {
        simd128::detect()
    }
}
