#![forbid(unsafe_op_in_unsafe_fn)]
#![no_std]
use core::{array, cmp, fmt, slice};

mod backend;
#[cfg(feature = "rand_core_0_6")]
pub mod rand_core_0_6;
mod scalar;
#[cfg(test)]
mod tests;

use arrayref::{array_mut_ref, array_ref};

pub use backend::Backend;

// Note: rustc's field reordering heuristc puts the buffer before the other fields because it has
// the highest alignment. There are other layouts that also minimize padding, but the one rustc
// picks happen to generate slightly better code for `next_u32` on some targets (e.g., on aarch64,
// it avoids computing the address of the buffer before checking if it needs to be refilled).
#[derive(Clone)]
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
#[derive(Clone)]
struct Buffer {
    words: [u32; 256],
}

impl Buffer {
    fn output(&self) -> &[u32; 248] {
        array_ref![&self.words, 0, 248]
    }

    fn output_mut(&mut self) -> &mut [u32; 248] {
        array_mut_ref![&mut self.words, 0, 248]
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

    fn need_refill(&self) -> bool {
        self.i >= self.buf.output().len()
    }

    pub fn next_u32(&mut self) -> u32 {
        // There doesn't seem to be a reliable, stable way to convince the compiler that this branch
        // is unlikely. For example, #[cold] on Backend::refill is ignored at the time of this
        // writing. Out of the various ways I've tried writing this function, this one seems to
        // generate the least bad assembly when compiled in isolation. (Of course, in practice we
        // want it to be inlined.)
        if self.need_refill() {
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

    pub fn read_bytes(&mut self, dest: &mut [u8]) {
        let mut total_bytes_read = 0;
        while total_bytes_read < dest.len() {
            let dest_remainder = &mut dest[total_bytes_read..];
            if self.need_refill() {
                self.refill();
            }
            let src_words = &mut self.buf.output_mut()[self.i..];
            let available_bytes = size_of_val::<[u32]>(src_words);
            let single_read_bytes = cmp::min(available_bytes, dest_remainder.len());

            // Round up the number of bytes read to whole 32-bit words. The + 3 can't overflow
            // because non-ZST slices are at most `isize::MAX` bytes large. Because `src.len()` is
            // always a multiple of four, this can only discards some bytes of RNG output when the
            // total size of `dest` is not a multiple of four.
            let single_read_words = (single_read_bytes + 3) / 4;
            debug_assert_eq!(single_read_bytes.div_ceil(4), single_read_words);

            // On little-endian targets this is a no-op and should easily be optimized out. On
            // big-endian targets it's necessary to get the same behavior as on little-endian
            // targets. We limit this to the words that will be read entirely or partially in this
            // iteration because that's less work. We do it before the read to get correct results
            // for the last few bytes of a read that's not a multiple of four bytes. Because the
            // other bytes of the last partially-read word are discarded in that case, we don't have
            // to undo the byte swap for that word after the read: the bytes that would be affects
            // will be skipped anyway.
            let src_words = &mut src_words[..single_read_words];
            for word in src_words.iter_mut() {
                *word = word.to_le();
            }
            // With the above preparations, the rest of the work is just a bytewise copy.
            dest_remainder[..single_read_bytes]
                .copy_from_slice(&words_as_ne_bytes(src_words)[..single_read_bytes]);

            total_bytes_read += single_read_bytes;
            self.i += single_read_words;
            debug_assert!(self.i <= self.buf.output().len());
            // We only ever read a word partially on the last iteration.
            debug_assert!(total_bytes_read == dest.len() || (single_read_bytes % 4 == 0));
        }
        debug_assert!(total_bytes_read == dest.len());
    }
}

fn words_as_ne_bytes<'a>(words: &'a [u32]) -> &'a [u8] {
    // This is almost certainly guaranteed, but spelling out again doesn't hurt.
    const _ALIGN_OK: () = assert!(align_of::<u8>() <= align_of::<u32>());

    let len = size_of_val::<[u32]>(words);
    let data: *const u8 = words.as_ptr().cast();
    // SAFETY:
    // * The pointer is valid for reading `len` bytes because `u32` has no padding and `words`
    //   covers exactly that many bytes.
    // * The pointer is non-null because it came from a valid `&[T]`.
    // * Alignment is OK because `u32` has equal or greater alignment as `u8`
    // * The memory will not be written for the duration of 'a because we have a shared borrow of
    //   `words` for the same duration.
    // * The total size in bytes is less than `isize::MAX` because it's the same number of bytes as
    //   `words`, so we just inherit that property.
    unsafe { slice::from_raw_parts::<'a, u8>(data, len) }
}

impl fmt::Debug for ChaCha8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(core::any::type_name::<Self>())
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
