//! High-quality non-cryptographic randomness with a specification and good performance.
//!
//! This crate implements the ChaCha8Rand algorithm, first designed for Go 1.22's
//! [`math/rand/v2`][go-blog]. It also has a [self-contained specification][spec] with sample
//! outputs.
//!
//! TODO: more blurb, introductory example
//!
//! # Why ChaCha8Rand?
//!
//! There are countless other random number generators. Like most of them, ChaCha8Rand promises high
//! quality output and good performance. In addition, it has two less common features. First, it has
//! a language-independent specification and test vectors, which supports long-term reproducibility.
//! Second, being built on top of a well-regarded cryptographic primitive means there is less doubt
//! and fewer caveats about its quality and robustness compared to generators that were not designed
//! to survive serious cryptanalysis.
//!
//! Thus, ChaCha8Rand may be a good choice if:
//!
//! 1. You need good statistical randomness with seeding, and want the relationship between the seed
//!    and the generated output to be stable in the long term and independent of a particular
//!    library's quirks.
//! 2. You want to do cute tricks like having a tree of RNGs where parents generate the seeds for
//!    their children, or generating multiple random seeds and expecting the output streams to be
//!    independent of each other. Most statistical generators either aren't designed to support such
//!    use cases at all, or only try to support specific usage patterns, so there's a risk of
//!    "holding it wrong" and getting output of sub-par quality.
//! 3. You've seen one too many "fast, high quality" generators turning out to be flawed. You would
//!    prefer the greater assurance of a cryptographically secure random number generator, but you
//!    actually need reproducibility from a seed and/or you're a little worried about the OS entropy
//!    source being a little slow or quirky on some machines your program may run on.
//!
//! ChaCha8Rand should not be used for cryptographic purposes (see below for reasons) but can
//! otherwise be used in a black-box fashion: seed goes in, unlimited randomness comes out. The
//! state space is so large that generating many seeds uniformly at random has negligible risk of
//! identical or overlapping output sequences. If there were any issues with the statistical quality
//! of its output, or if the output sequences for different seeds (with some known relation between
//! them) were not statistically independent, that would most likely imply a major breakthrough in
//! the cryptanalysis of ChaCha20 as a stream cipher. Since the eight-round variant has survived
//! significant cryptanalysis, it's probably safe to assume that there are no serious flaws waiting
//! to be discovered that would be relevant for non-cryptographic randomness.
//!
//! # Don't Use This For Cryptography
//!
//! Although the algorithm uses a stream cipher as building block, it is not a replacement for
//! cryptographically secure randomness from the operating system or crypto libraries that wrap it.
//! As Russ Cox and Filippo Valsorda put it [when introducing the algorithm][go-blog]:
//!
//! > It’s still better to use crypto/rand, because the operating system kernel can do a better job
//! > keeping the random values secret from various kinds of prying eyes, the kernel is continually
//! > adding new entropy to its generator, and the kernel has had more scrutiny. But accidentally
//! > using math/rand is no longer a security catastrophe.
//!
//! In addition, this crate makes design decisions not conductive to security:
//!
//! * It generates output in relatively large batches, and does not scrub data from the buffer
//!   immediately after it is consumed.
//! * It has no APIs for seeding directly from OS-provided entropy, deterministic seeding is the
//!   path of least resistance.
//! * It supports copying and (de-)serialization of the RNG state, which is a big footgun in
//!   cryptographic applications. Also, it does not implement fast key erasure, instead keeping the
//!   last key around longer to support more compact state serialization.
//! * It contains some unsafe code for accessing `core::arch` intrinsics, including some extra
//!   complications for runtime feature detection, and none of this has been audited or reviewed.
//!   I've tried to follow best practices w.r.t. encapsulating and justifying every unsafe
//!   operation, and Miri has no issue with the tests that it can run. Still, a security-minded
//!   project should demand more scrutiny or prefer an implementation with less `unsafe`.
//!
//! # Features and Drawbacks
//!
//! This crate has SIMD backends for better preformance on several common targets: SSE2 and AVX2 on
//! x86 and x86_64, NEON on AArch64 (little-endian only), and SIMD128 on Webassembly. Only the AVX2
//! backend uses runtime feature detection. Of course, there is also a portable implementation for
//! all other platforms, which is slower in microbenchmarks but still plenty fast enough for most
//! use cases. Other features include:
//!
//! * It's tested to work correctly on big endian targets.
//! * RNG state can be serialized efficiently into 9x4 = 36 bytes.
//! * Optional implementations for `rand_core` traits behind a Cargo feature flag.
//! * The crate is `no_std` except when required for runtime feature detection (only x86). Runtime
//!   detection may become optional (via a Cargo feature) in the future, unless feature detection in
//!   `core` becomes available first.
//!
//! The main reasons why you might not want to use this crate are the use of `unsafe` for accessing
//! SIMD intrinsics and the relatively large buffer (4x larger than the Go implementation). The
//! latter means each RNG instance is a little over a thousand bytes large, which may be an issue if
//! you want to have many instances and care about memory consumption and/or only consume a small
//! amount of randomness from most of those instances.
//!
//! # Output Stability And Go Interoperability
//!
//! This crate and Go's `rand.ChaCha8` both implement [the same specification][spec], so they (and
//! any other implementation of the spec) should produce the same stream of bytes from a given seed.
//! Note that the C2SP specification hasn't received a "v1.0" tag yet at the time of this writing,
//! but since Go has already shipped its implementation a while ago and the same people wrote the
//! spec, incompatible changes seem unlikely.
//!
//! This crate may still go through breaking API changes before its 1.0 release. However, the output
//! behavior described by the specification will not change unless unless the specification itself
//! makes a breaking 2.0 release. Thus, this crate promises a deterministic, portable output byte
//! stream across minor and patch versions (except fixing divergences from the spec if any are found
//! before the 1.0 release). Compare and contrast the [reproducibility policy of the `rand`
//! crates][rand-repro-policy]. Specifically:
//!
//! 1. For a given 32-byte seed, if you consume the output as a byte stream with calls to
//!    [`ChaCha8Rand::read_bytes`], you'll the unique byte stream described by the spec, no matter
//!    how you slice it up into multiple reads of possibly different sizes. This should match Go's
//!    `ChaCha8.Read`, added in Go 1.23.
//! 2. For a given 32-byte seed, if you consume the output as sequence of `u64`s, the raw output
//!    bytes are always interpreted as little-endian integers. This is also how Go's `Uint64()`
//!    method works, at least right now.
//! 3. The same applies, if you consume the output as a sequence of `u32`s. Note that there's no
//!    direct Go equivalent for this (the `Rand` helper has a method for this, but it draws 64 bits
//!    from the source).
//! 3. All inputs (seeds) and outputs always use little-endian byte order. ChaCha20 works on 32-bit
//!    words internally but the "native" endianness never affects anything. Neither does
//!    `size_of::<usize>()` for that matter, because it doesn't show up in the API.
//!
//! However, if you mix or interleave different ways of drawing randomness from a single generator,
//! the output you get is not covered by this promise. The byte-oriented `Read` method of Go's
//! `ChaCha8` has this note in its documentation:
//!
//! > If calls to Read and Uint64 are interleaved, the order in which bits are returned by the two
//! > is undefined, and Read may return bits generated before the last call to Uint64.
//!
//! Similarly, interleaving calls to [`ChaCha8Rand::read_bytes`], [`ChaCha8Rand::read_u64`], and
//! [`ChaCha8Rand::read_u32`] may produce output bytes out of order w.r.t. the raw byte stream and
//! and some might be skipped (but each byte will be output at most once). At the time of this
//! writing [`ChaCha8Rand`] and Go already behave very differently when you do this. Further changes
//! to this behavior will not be considered semver-breaking.
//!
//! [go-blog]: https://go.dev/blog/chacha8rand
//! [spec]: https://c2sp.org/chacha8rand
//! [rand-repro-policy]: https://rust-random.github.io/book/crate-reprod.html
#![forbid(unsafe_op_in_unsafe_fn)]
#![no_std]
use core::{array, cmp, error::Error, fmt};

use arrayref::array_ref;

mod backend;
mod common_guts;
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
// picks happen to generate slightly better code for `read_{u32,u64}` on some targets (e.g., on
// aarch64, not computing the address of the buffer before checking if it needs to be refilled).
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
    #[inline]
    fn output(&self) -> &[u8; BUF_OUTPUT_LEN] {
        array_ref![&self.bytes, 0, BUF_OUTPUT_LEN]
    }

    #[inline]
    fn new_key(&self) -> &[u8; 32] {
        array_ref![&self.bytes, BUF_OUTPUT_LEN, 32]
    }
}

pub struct Seed(pub [u32; 8]);

impl From<[u32; 8]> for Seed {
    #[inline]
    fn from(words: [u32; 8]) -> Self {
        Self(words)
    }
}

impl From<[u8; 32]> for Seed {
    #[inline]
    fn from(bytes: [u8; 32]) -> Self {
        Self(array::from_fn(|i| {
            u32::from_le_bytes(*array_ref![bytes, 4 * i, 4])
        }))
    }
}

impl From<&[u8; 32]> for Seed {
    #[inline]
    fn from(bytes: &[u8; 32]) -> Self {
        Self::from(*bytes)
    }
}

impl From<&[u32; 8]> for Seed {
    #[inline]
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

    #[inline]
    pub fn read_u32(&mut self) -> u32 {
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

    #[inline]
    pub fn read_u64(&mut self) -> u64 {
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
