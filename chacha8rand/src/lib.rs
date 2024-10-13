//! Reproducible, robust and (last but not least) fast pseudorandomness.
//!
//! This crate implements the [ChaCha8Rand][spec] specification, originally designed for Go's
//! `math/rand/v2` package. The language-independent specification and test vector helps with
//! long-term reproducibility and interoperability. Building on the ChaCha8 stream cipher ensures
//! high statistical quality and removes entire classes of "you're holding it wrong"-style problems
//! that lead to sub-par output. It's also carefully designed and implemented (using SIMD
//! instructions when available) to be so fast that it shouldn't ever be a bottleneck. However, it
//! [should not be used for cryptography](#no-crypto).
//!
//! # Quick Start
//!
//! In the interest of simplicity and reproducibility, there's no global or thread-local generator.
//! You'll always have to pick a 32-byte seed yourself, create a [`ChaCha8Rand`] instance from it,
//! and pass it around in your program. Usually, you'll generate an unpredictable seed at startup by
//! default, but store or log it somewhere and support running the program again with the same seed.
//! For the first half, it's usually best to provide a full 256 bits of entropy via the
//! [`getrandom`][getrandom] crate:
//!
//! ```
//! # use chacha8rand::ChaCha8Rand;
//! let mut seed = [0; 32];
//! getrandom::getrandom(&mut seed).expect("getrandom failure is 'highly unlikely'");
//! let mut rng = ChaCha8Rand::new(&seed);
//! // Now we can make random choices
//! let heads_or_tails = if rng.read_u32() & 1 == 0 { "heads" } else { "tails" };
//! println!("The coin came up {heads_or_tails}.");
//! ```
//!
//! The best place and format to store the seed will vary, but 64 hex digits is a good default
//! because it can be copied and pasted as (technically) human-readable text. However, if you want
//! to let humans *pick a seed by hand* for any reason, then asking them for exactly 64 hex digits
//! would be a bit rude. For such cases, it's more convenient to accept an UTF-8 string and feed it
//! into a hash function with 256 bit output, such as SHA-256 or Blake3.
//!
//! In any case, once you've created a [`ChaCha8Rand`] instance with an initial seed, you can
//! consume its output as a sequence of bytes or as stream of 32-bit or 64-bit integers. If you need
//! support for other types, for integers in a certain interval, or other distributions, you might
//! want to enable the [crate feature](#crate-features) to combine [`ChaCha8Rand`] with the `rand`
//! crate. Another thing you can do (even without `rand`) is deriving seeds for multiple sub-RNGs
//! that are used for different purposes, without creating correlation between those different
//! streams of randomness (e.g., for [roguelike games][sts-corr-rand]). The ability to do this is
//! one reason why I wrote this crate, and there's a convenience method for it:
//!
//! ```
//! # use chacha8rand::ChaCha8Rand;
//! # let initial_seed = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
//! let mut seed_gen = ChaCha8Rand::new(&initial_seed);
//! // Create new instances with seeds from `seed_gen`...
//! let mut rng1 = ChaCha8Rand::new(&seed_gen.read_seed());
//! let mut rng2 = ChaCha8Rand::new(&seed_gen.read_seed());
//! assert_ne!(rng1.read_u64(), rng2.read_u64(), "if this fails you're _very_ unlucky");
//! // ... and/or re-seed an existing instance in-place:
//! rng1.set_seed(&seed_gen.read_seed());
//! ```
//!
//! Note that using the output of a statistical RNG to seed other instances of the same algorithm
//! (or a related one) is often risky or outright broken. Even generators that explicitly support
//! it, like SplitMix, often distinguish "generate a new seed" from ordinary random output.
//! ChaCha8Rand has no such caveats: its state space is so large, and its output is of such high
//! quality, that there's no risk of creating overlapping output sequences or correlations between
//! generators seeded this way. Indeed, every instance regularly replaces its current seed with some
//! of its own output. Using the rest of the output as seeds for other instances works just as well.
//!
//! # <a name="no-crypto"></a> Don't Use This For Cryptography
//!
//! ChaCha8Rand derives its high quality from eight-round ChaCha20, which is a secure stream cipher
//! as far as anyone knows today (but in most cases you also want ciphertext authenticity, i.e., an
//! AEAD mode). Thus, ChaCha8Rand can mostly be used as a black-box source of high quality
//! pseudorandomness. If there were any patterns or biases in its output, or if the output sequences
//! for different seeds (with some known relation between them) were not statistically independent,
//! that would most likely imply a major breakthrough in the cryptanalysis of ChaCha20. However,
//! that doesn't mean this crate is a replacement for cryptographically secure randomness from the
//! operating system or libraries that wrap it, such as [`getrandom`][getrandom].
//!
//! As Russ Cox and Filippo Valsorda wrote [while introducing the algorithm][go-blog], regarding
//! accidental use of Go's `math/rand` to generate cryptographic keys and other secrets:
//!
//! > Using Go 1.20, that mistake is a serious security problem that merits a detailed investigation
//! > to understand the damage. [...] Using Go 1.22, that mistake is just a mistake. It’s still
//! > better to use crypto/rand, because the operating system kernel can do a better job keeping the
//! > random values secret from various kinds of prying eyes, the kernel is continually adding new
//! > entropy to its generator, and the kernel has had more scrutiny. But accidentally using
//! > math/rand is no longer a security catastrophe.
//!
//! Keep in mind that Go has a global generator which is seeded from OS-provided entropy on startup.
//! If you pick a seed yourself (which you always do when using this crate), the output of the
//! generator is at best as unpredictable as that seed was. There are also other design decisions in
//! this implementations that would be inappropriate for security-sensitive applications. For
//! example, it doesn't handle process forking or VM image cloning, it doesn't even try to scrub
//! generated data from its internal buffer after it's consumed, and it sacrifices so-called *fast
//! key erasure* in favor of needing fewer bytes to serialize the current state.
//!
//! # <a name="crate-features"></a> Crate Features
//!
//! The crate is `no_std` and "no `alloc`" by default. There are two crate features you might enable
//! when you add `chacha8rand` to your Cargo.toml file:
//!
//! * **`std`**: opts out of `#![no_std]`, enables runtime detection of `target_feature`s for higher
//!   performance on some targets. It does not affect the API surface, so ideally libraries leave
//!   this decision to the top-level binary. Most applications should probably enable it because
//!   it's basically a free performance improvement.
//! * **`rand_core_0_6`**: implement the `RngCore` and `SeedableRng` traits from `rand_core` v0.6,
//!   for integration with (that version of) the rand ecosystem. When new semver-incompatible
//!   versions of `rand` and friends get released, they will get another feature flag so both sets
//!   of impls can coexist.
//!
//! Neither feature is enabled by default, so you don't need to add `no-default-features = true`. In
//! fact, please don't, because that makes it harder to add more feature flags in the future without
//! semver-breaking changes. There are also some features with an "unstable" prefix in their name.
//! Anything covered by "unstable" features is explicitly not covered by SemVer and may change or be
//! removed at any time.
//!
//! As for the non-Cargo meaning of "features", take a look at [`ChaCha8Rand`] to learn more about
//! these aspects:
//!
//! * On several common targets, SIMD instructions are used to improve performance.
//! * Once the crate reaches 1.0, the output you get from a given seed will not change in minor or
//!   patch releases.
//! * The generator's state can be serialized into as little as 34 bytes.
//!
//! # Drawbacks
//!
//! The main reasons why you might not want to use this crate are the use of `unsafe` for accessing
//! SIMD intrinsics and the relatively large buffer (4x larger than the Go implementation). The
//! latter means each RNG instance is a little over a thousand bytes large, which may be an issue if
//! you want to have many instances and care about memory consumption and/or only consume a small
//! amount of randomness from most of those instances.
//!
//! [getrandom]: https://crates.io/crates/getrandom
//! [go-blog]: https://go.dev/blog/chacha8rand
//! [spec]: https://c2sp.org/chacha8rand
//! [sts-corr-rand]: https://forgottenarbiter.github.io/Correlated-Randomness/
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

#[cfg(feature = "unstable_internals")]
pub use backend::Backend;
#[cfg(not(feature = "unstable_internals"))]
use backend::Backend;

const BUF_TOTAL_LEN: usize = 1024;
const BUF_OUTPUT_LEN: usize = BUF_TOTAL_LEN - 32;

/// A deterministic stream of pseudorandom data from a 32-byte seed.
///
/// See the crate documentation for a higher-level introduction and quick-start examples. Here
/// you'll only find excessive extra details about reproducibility and some notes about
/// (de-)serialization and SIMD backends.
///
/// # Reproducibility
///
/// Until I release version 1.0 of this crate, I reserve the right to fix divergences from the
/// [ChaCha8Rand specification][spec] or tweak behavior not covered by the spec. Afterwards, **the
/// output for a given seed will not change in minor or patch releases**. If there is an
/// incompatible change to the spec and I want to implement it, or someone finds a serious bug that
/// can't be fixed without changing output, that will be a semver-major release. Note that the spec
/// technically hasn't been tagged as 1.0, but breaking changes seem very unlikely since the same
/// people already shipped an implementation in the Go standard library.
///
/// The sequence of bytes generated from a given seed is (or should be) uniquely determined by the
/// spec. It does not depend on the target platform, in particular, it doesn't depend on "native"
/// endianness. The algorithm uses 32-bit words internally, but it always interprets the seed in
/// little-endian byte order and produces output in little-endian byte order. So if you only ever
/// treat the generator as a stream of bytes ([`ChaCha8Rand::read_bytes`] and
/// [`ChaCha8Rand::read_seed`]), you'll get the same output from any other implementation of the
/// spec.
///
/// Alternatively, if you treat it as a stream of 64-bit integers ([`ChaCha8Rand::read_u64`]), every
/// group of eight bytes is interpreted in little-endian byte order. This is not part of the spec,
/// strictly speaking, but it's a fairly natural choice. It's also how Go's version implements the
/// `Uint64()` method and the `rand.Source` interface.
///
/// However, if you start consuming the randomness in one way and later switch to another way, the
/// output you'll get depends on implementation choices that are not uniquely determined. The raw
/// byte stream may be consumed out of order and parts of it may be skipped, but no byte should be
/// used more than once. This crate also lets you consume 32-bit integers
/// ([`ChaCha8Rand::read_u32`]) directly, which has no Go equivalent (the `Rand.Uint32` helper takes
/// 64 bits from the source and discards half). I can only document what this crate implements:
///
/// * Consuming bytes in any granularity via [`ChaCha8Rand::read_bytes`] and
///   [`ChaCha8Rand::read_seed`] always consumes the output byte stream in order, without skipping
///   or reordering anything.
/// * Consuming integers with [`ChaCha8Rand::read_u32`] and [`ChaCha8Rand::read_u64`] generally acts
///   like reading the corresponding number of bytes (`size_of::<T>()`) with
///   [`ChaCha8Rand::read_bytes`] and interpreting them in little-endian byte order, *except* when
///   there are too few output bytes left in the current iteration of ChaCha8Rand (992 output bytes
///   plus 32 byte input for the next iteration). In this case, the remaining bytes of the current
///   iteration are skipped and the output is taken from the first bytes of the next iteration.
///
/// Committing to this behavior effectively means baking in some artifacts of the current
/// implementation, e.g., buffering a full iteration of output and and handling unaligned
/// `u32`/`u64` reads from the buffer. Again, I reserve the right to tweak this before the crate's
/// 1.0 release, but then I'll commit to *something*.
///
/// # Serialization and Deserialization
///
/// Besides storing the initial seed, you can also store the state of the generator at any point in
/// time with [`ChaCha8Rand::clone_state`] and [`ChaCha8Rand::try_restore_state`]. See those methods
/// for more details. The important thing with respect to reproducibility is that the serialized
/// state records an exact position in the output byte stream. Thus, if you save the state at any
/// point and later restore it, you'll get the same output as if you had kept working with the
/// original generator, regardless of how how you read from it before and after.
///
/// # SIMD Backends
///
/// Like the Go version, this crate uses 128-bit SIMD for better performance on x86_64 (SSE2
/// instructions) and AArch64 (NEON, [little-endian only for now][aarch64be-neon]). Of course, there
///  is also a portable implementation for all other platforms, which is slower in microbenchmarks
/// but still plenty fast enough for most use cases.
///
/// Unlike Go 1.23, this crate also uses SIMD on 32-bit x86 targets and Webassembly with the
/// `simd128` feature. There's also a AVX2 backend for 256-bit SIMD on x86 and x86_64. This backend
/// uses runtime feature detection (if the `std` feature is enabled) so you don't have to fiddle
/// with `-Ctarget-feature` and risk the program not working on some older CPUs. Other instruction
/// sets and more runtime feature detection may be added in the future.
///
/// [aarch64be-neon]: https://github.com/rust-lang/stdarch/issues/1484
/// [spec]: https://c2sp.org/chacha8rand
#[derive(Clone)]
pub struct ChaCha8Rand {
    // Note: rustc's field reordering heuristc puts the buffer before the other fields because it has
    // the highest alignment. There are other layouts that also minimize padding, but the one rustc
    // picks happen to generate slightly better code for `read_{u32,u64}` on some targets (e.g., on
    // aarch64, not computing the address of the buffer before checking if it needs to be refilled).
    backend: Backend,
    seed: [u32; 8],
    /// Position in `buf.output()` of the next byte to produce as output. Should be equal to
    /// [`BUF_OUTPUT_LEN`] if all the output was already consumed. Larger values are not canonical
    /// and *shouldn't* occur, but we never rely on this for safety and most other code also tries
    /// to handle larger values gracefully.
    bytes_consumed: usize,
    buf: Buffer,
}

impl fmt::Debug for ChaCha8Rand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChaCha8Rand { .. }")
    }
}

#[derive(Clone, Copy)]
pub struct ChaCha8State {
    pub seed: [u8; 32],
    pub bytes_consumed: u16,
}

impl fmt::Debug for ChaCha8State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChaCha8State { .. }")
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
    #[inline]
    pub fn new(seed: &[u8; 32]) -> Self {
        // On x86, we prefer AVX2 over SSE2 when both are available. The other SIMD backends aren't
        // really ordered by preference because they're for mutually exclusive target platforms, but
        // it's less of a mess to chain them like this than to replicate the `cfg` soup. We only use
        // the scalar backend if none of the SIMD backends are available.
        let backend = avx2::detect()
            .or_else(sse2::detect)
            .or_else(neon::detect)
            .or_else(simd128::detect)
            .unwrap_or_else(scalar::backend);
        Self::with_backend_impl(seed, backend)
    }

    #[cfg(feature = "unstable_internals")]
    #[inline]
    pub fn with_backend(seed: &[u8; 32], backend: Backend) -> Self {
        Self::with_backend_impl(seed, backend)
    }

    fn with_backend_impl(seed: &[u8; 32], backend: Backend) -> Self {
        let mut this = ChaCha8Rand {
            seed: [0; 8],
            bytes_consumed: 0,
            buf: Buffer { bytes: [0; 1024] },
            backend,
        };
        this.set_seed(seed);
        this
    }

    pub fn set_seed(self: &mut ChaCha8Rand, seed: &[u8; 32]) {
        self.seed = seed_from_bytes(seed);
        // Fill the buffer immediately because we want the next bytes of output to come directly
        // from the new seed, not from the old seed or from the seed *after* `seed`.
        self.backend.refill(&self.seed, &mut self.buf);
        self.bytes_consumed = 0;
    }

    pub fn clone_state(&self) -> ChaCha8State {
        // The cast to u16 can't truncate because we never set the field to anything larger than the
        // size of the output buffer. But if that does happen, restoring from the resulting state
        // could behave incorrectly. That code path is also careful about it but defense in depth
        // can't hurt, so let's saturate here.
        debug_assert!(self.bytes_consumed <= BUF_OUTPUT_LEN);
        let bytes_consumed = cmp::min(self.bytes_consumed, BUF_OUTPUT_LEN) as u16;
        ChaCha8State {
            seed: seed_to_bytes(&self.seed),
            bytes_consumed,
        }
    }

    pub fn try_restore_state(&mut self, state: &ChaCha8State) -> Result<(), RestoreStateError> {
        // We never produce `bytes_consumed` values larger than the output buffer's size, so we
        // don't accept it either.
        let bytes_consumed = usize::from(state.bytes_consumed);
        if bytes_consumed > BUF_OUTPUT_LEN {
            return Err(RestoreStateError { _private: () });
        }

        // We can just use `set_seed` to fill the buffer and then skip the parts of that chunk that
        // were marked as already consumed by adjusting our position in the refilled buffer.
        self.set_seed(&state.seed);
        self.bytes_consumed = bytes_consumed;
        Ok(())
    }

    #[inline]
    fn refill(&mut self) {
        self.seed = seed_from_bytes(self.buf.new_key());
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

    pub fn read_seed(&mut self) -> [u8; 32] {
        let mut seed = [0; 32];
        self.read_bytes(&mut seed);
        seed
    }
}

fn seed_from_bytes(bytes: &[u8; 32]) -> [u32; 8] {
    array::from_fn(|i| u32::from_le_bytes(*array_ref![bytes, 4 * i, 4]))
}

fn seed_to_bytes(seed: &[u32; 8]) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (i, word) in seed.iter().enumerate() {
        bytes[4 * i..][..4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
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
    // This backend uses dynamic feature detection, so it's disabled in no_std mode and only gated
    // on `target_arch`. In theory it could also be enabled in no_std mode when AVX2 is statically
    // enabled, but that would probably complicate some unsafe code which seems like a bad trade.
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), feature = "std"))]
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

// These methods only exist to enable the benchmark (compiled as separate crate) to override backend
// selection and compare performance. It's not in the `backend` module to minimize that code that
// has to worry about upholding `Backend`'s invariant.
#[cfg(feature = "unstable_internals")]
impl Backend {
    pub fn scalar() -> Self {
        scalar::backend()
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
