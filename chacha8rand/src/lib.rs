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
//! use chacha8rand::ChaCha8Rand;
//!
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
//! streams of randomness (e.g., for roguelike games). The ability to do this is one reason why I
//! wrote this crate, and there's a convenience method for it:
//!
//! ```
//! use chacha8rand::ChaCha8Rand;
//!
//! let initial_seed = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
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
//! ChaCha8Rand derives its high quality from ChaCha8, which is a secure stream cipher as far as
//! anyone knows today (although in most cases you also want ciphertext authenticity, i.e., an AEAD
//! mode). Thus, ChaCha8Rand can mostly be used as a black-box source of high quality
//! pseudorandomness. If there were any patterns or biases in its output, or if the output sequences
//! for different seeds (with some known relation between them) were not statistically independent,
//! that would most likely imply a major breakthrough in the cryptanalysis of ChaCha. However, that
//! doesn't mean this crate is a replacement for cryptographically secure randomness from the
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
//! The crate is `no_std` and "no `alloc`" by default. There are currently two crate features you
//! might enable when depending on `chacha8rand`. You can manually add them to Cargo.toml (`features
//! = [...]` key) or use a command like `cargo add chacha8rand -F rand_core_0_6`. The features are:
//!
//! * **`std`**: opts out of `#![no_std]`, enables runtime detection of `target_feature`s for higher
//!   performance on some targets. It does not (currently) affect the API surface, so ideally
//!   libraries leave this decision to the top-level binary. For forward compatibility, enabling
//!   this feature *always* adds a dependency on `std`, even on targets where `std` isn't needed
//!   today.
//! * **`rand_core_0_6`**: implement the `RngCore` and `SeedableRng` traits from `rand_core` v0.6,
//!   for integration with `rand` version 0.8. The upcoming semver-incompatible release of the rand
//!   crates (v0.9) will get another feature so that `ChaCha8Rand` can implement both the new and
//!   the old versions of these traits at the same time.
//!
//! Neither feature is enabled by default, so you don't need `no-default-features = true` / `cargo
//! add --no-default-features`. In fact, please don't, because then your code might break if a later
//! version moves existing functionality under a new on-by-default feature.
//!
//! There are also some features with an "unstable" prefix in their name. Anything covered by these
//! is for internal use only (e.g., the crate's benchmarks are compiled as a separate crate) and
//! explicitly not covered by SemVer.
//!
//! # Minimum Supported Rust Version (MSRV)
//!
//! There is no MSRV policy at the moment, so features from new stable Rust versions may be adopted
//! as soon as they come out (but in practice I don't expect to make frequent releases). If you need
//! to use this crate with a specific older version, you can open an issue and we can take a look at
//! how easy or difficult it would be to support that version.
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
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![no_std]
use core::{array, cmp, error::Error, fmt};

// Currently, we only *need* `std` on x86 for runtime feature detection. But later versions might
// use runtime detection on more platforms, or implement traits that require `std`. It would suck if
// a semver-minor update like that broke something because people (like myself) were using the crate
// with the `std` feature enabled in a `#![no_std]` binary. So we always pull in the crate here.
#[cfg(feature = "std")]
extern crate std;

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

/// A deterministic stream of pseudorandom bytes from a 32-byte seed.
///
/// See the crate documentation for a higher-level introduction and quick-start examples. Here
/// you'll only find excessive extra details about reproducibility and some notes about
/// (de-)serialization and SIMD backends.
///
/// This type implements traits from the rand crate (`RngCore` and `SeedableRng`), but you need to
/// [opt-in with a feature flag][crate-features] to use those impls.
///
/// # <a name="repro-details"></a> Reproducibility
///
/// The [ChaCha8Rand specification][spec] describes how a seed is expanded into an unbounded stream
/// of pseudorandom bytes. This stream should be uniquely determined: byte order is fixed to little
/// endian, the differences between various ChaCha20 variants (32- or 64-bit counter, nonce size)
/// don't matter in this context, and the test vector included in the spec should remove any
/// remaining doubts.
///
/// Until the 1.0 release of this crate, I reserve the right to make API breaking changes and fix
/// bugs even if they change the output. But the intent is to match the spec precisely and not
/// change anything about the output for a given seed in future releases. If the spec gets an
/// incompatible 2.0 release and I want to implement it, that will be a semver-major release. Note
/// that the spec technically hasn't been tagged as 1.0, but breaking changes seem very unlikely
/// since the same people already shipped an implementation in the Go standard library.
///
/// Besides treating the generator as a byte stream with [`ChaCha8Rand::read_bytes`], you can also
/// use other methods such as [`ChaCha8Rand::read_u64`]. What happens when you interleave calls to
/// these methods, i.e., mix and match different read granularities? There's no clear "best" answer.
/// Different implementation strategies lead to different behavior and it's reasonable to not
/// specify it or reserve the right to change it later. However, for this crate I wanted to commit
/// to a simple and useful mental model. What I ended up with is:
///
/// * The generator is *just* the spec-mandated stream of bytes. Repeatedly calling `read_bytes`
///   gives you these bytes in order without skipping, reordering, or duplicating anything.
/// * The number of calls to `read_bytes` and the size of each read doesn't affect behavior. The
///   number of bytes consumed is never rounded up internally because that would skip some bytes.
///   Zero-sized reads are no-ops.
/// * Methods like `read_u32`, `read_u64`, `read_seed`, and any others that might be added in the
///   future, behave exactly like reading the appropriate number of bytes from the stream and
///   converting those to the result type. When byte order matters, this always uses little endian.
///
/// This is different from what Go's implementation does when you interleave calls to its `Uint64`
/// and `Read` methods. The documentation explicitly says the results are unspecified and may return
/// bytes "out of order". The implementation in Go 1.23 does in fact behave differently from this
/// crate in many cases. (It also doesn't provide a direct way to read a 32-bit integer.)
///
/// # Serialization and Deserialization
///
/// Besides storing the initial seed, you can also snapshot state of the generator at any point in
/// time with [`ChaCha8Rand::clone_state`] and [`ChaCha8Rand::try_restore_state`]. See
/// [`ChaCha8State`] for more details. The important thing with respect to reproducibility is that
/// the serialized state records an exact position in the output byte stream. Thus, if you save the
/// state at any point and later restore it, you'll get the same output as if you had kept working
/// with the original generator, regardless of how how you read from it before and after.
///
/// # SIMD Backends
///
/// Like the Go version, this crate uses 128-bit SIMD for better performance on x86_64 (SSE2
/// instructions) and AArch64 (NEON, [little-endian only for now][aarch64be-neon]). Of course, there
///  is also a portable implementation for all other platforms, which is slower in microbenchmarks
/// but still plenty fast enough for most use cases.
///
/// Unlike Go (version 1.23), this crate also uses SIMD on 32-bit x86 targets and Webassembly with
/// the `simd128` feature. There's also a AVX2 backend for 256-bit SIMD on x86 and x86_64. This
/// backend uses runtime feature detection (if the `std` feature is enabled) so you don't have to
/// fiddle with `-Ctarget-feature` and risk the program not working on some older CPUs. Other
/// instruction sets and more runtime feature detection may be added in the future.
///
/// [aarch64be-neon]: https://github.com/rust-lang/stdarch/issues/1484
/// [crate-features]: ./index.html#crate-features
/// [spec]: https://c2sp.org/chacha8rand
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

impl fmt::Debug for ChaCha8Rand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChaCha8Rand { .. }")
    }
}

/// Snapshot of the state of a [`ChaCha8Rand`] instance.
///
/// Created with [`ChaCha8Rand::clone_state`] and used by [`ChaCha8Rand::try_restore_state`]. It
/// simply records the seed of the current iteration of the generator and how many output bytes of
/// that iteration have already been consumed. Restoring from it is effectively the same as calling
/// `rng.set_seed(&seed)` and then throwing away `bytes_consumed` many bytes of output. However,
/// going through [`ChaCha8Rand::try_restore_state`] can catch some possible mistakes because it
/// validates that `bytes_consumed` is in the range it should be.
///
/// Possible use cases include:
///
/// * Saving and restoring the RNG state as part of a game's (auto-)save feature.
/// * Suspending and later resume a long-running computation by saving the RNG (and all other state)
///   to disk.
/// * Forking a randomized algorithm, running it twice with the same randomness but handling
///   different input, to see how they diverge (e.g., "what if" queries).
///
/// There are no `serde` impls. Instead, the fields are public so you can (de-)serialize them in any
/// way you see fit. In this case you should be prepared to handle errors due to out-of-range
/// `bytes_consumed` values gracefully.
///
/// Nothing stops you from constructing a [`ChaCha8State`] out of thin air (rather than cloning from
/// an existing generator), but there's probably no reason to do so. You can supply a new seed
/// directly with [`ChaCha8Rand::new`] or [`ChaCha8Rand::set_seed`].
///
/// Finally, note that [`ChaCha8Rand`] also implements `Clone`. Cloning a generator achieves the
/// same effect as taking a snapshot of its state and restoring from it, but the generator is much
/// larger because it includes a big buffer of output. If you want to duplicate a generator and
/// consume output from both copies, cloning is easier *and* doesn't have to re-compute the output
/// that's already buffered. But if you store several snapshots and *possibly* use some of them at a
/// later time, cloning would waste a lot of memory.
///
/// # Examples
///
/// ```
/// # use chacha8rand::ChaCha8Rand;
/// # let mut rng = ChaCha8Rand::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456");
/// let state = rng.clone_state();
/// let first_output = rng.read_u64();
/// rng.try_restore_state(&state).expect("snapshot is valid because it was not modified");
/// assert_eq!(rng.read_u64(), first_output);
/// ```
#[derive(Clone, Copy)]
pub struct ChaCha8State {
    /// The seed of the current ChaCha8Rand iteration.
    pub seed: [u8; 32],
    /// How much output from the current ChaCha8Rand iteration was already consumed.
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

/// Error returned from [`ChaCha8Rand::try_restore_state`] for corrupted snapshots.
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
    /// Create a new generator from the given seed.
    ///
    /// This will eagerly generates data to fill the generator's internal buffer. Therefore, it may
    /// be a bit wasteful to call if you won't actually need any output from the generator. Don't
    /// over-complicate your program to avoid that, but keep it in mind if in case it's easy to
    /// avoid.
    ///
    /// # Examples
    ///
    /// Reproducing the sample output from [the ChaCha8Rand specification]:
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// let mut sample = ChaCha8Rand::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456");
    /// assert_eq!(sample.read_u64(), 0xb773b6063d4616a5);
    /// assert_eq!(sample.read_u64(), 0x1160af22a66abc3c);
    /// assert_eq!(sample.read_u64(), 0x8c2599d9418d287c);
    /// // ... and so on
    /// ```
    ///
    /// [spec]: https://c2sp.org/chacha8rand
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
    #[allow(
        missing_docs,
        reason = "internal API only exposed unstably for benchmarks"
    )]
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

    /// Reset this generator, as if overwriting it with `ChaCha8Rand::new(seed)`.
    ///
    /// This helper method exists is because it's more convenient sometimes and might avoid copying
    /// a relatively large type from one location to another.
    ///
    /// # Examples
    ///
    /// Restoring the original seed after a simulation to reproduce it:
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// # let initial_seed = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
    /// fn run_simulation(rng: &mut ChaCha8Rand) -> String {
    ///     // TODO: figure out the details
    ///     format!("the meaning of life is {}", rng.read_u64())
    /// }
    ///
    /// let mut rng = ChaCha8Rand::new(&initial_seed);
    /// let result = run_simulation(&mut rng);
    /// println!("Simulation says {result} - let's try to replicate that");
    /// rng.set_seed(&initial_seed);
    /// let result_again = run_simulation(&mut rng);
    /// assert_eq!(result, result_again);
    /// ```
    pub fn set_seed(self: &mut ChaCha8Rand, seed: &[u8; 32]) {
        self.seed = seed_from_bytes(seed);
        // Fill the buffer immediately because we want the next bytes of output to come directly
        // from the new seed, not from the old seed or from the seed *after* `seed`.
        self.backend.refill(&self.seed, &mut self.buf);
        self.bytes_consumed = 0;
    }

    /// Consume four bytes of uniformly random data and return them as `u32`.
    ///
    /// This is always equivalent to [`ChaCha8Rand::read_bytes`] plus `u32::from_le_bytes`, but 99%
    /// of the time it's more efficient. If you simply need 32 or fewer uniformly random bits, this
    /// method enables this conveniently and without involving the `rand_*` crates.
    ///
    /// On the other hand, if you want integers in a range like `0..n` or `m..=n`, you should *not*
    /// use this method and combine it with the remainder operator `%`. The `rand` crate has
    /// convenient and efficient APIs for doing that correctly, without introducing bias. It also
    /// supports more data types, non-uniform distributions, and higher-level operations such as
    /// shuffling lists. You can use it with ChaCha8Rand by [activating the crate
    /// feature][rand-feature] so that [`ChaCha8Rand`] implements the rand traits. See the examples
    /// for more details.
    ///
    /// # Examples
    ///
    /// To generate integers in some range `0..n` or `0..=n`, or to generate other types such as
    /// floating point numbers, combine [`ChaCha8Rand`] with the rand crate (or another
    /// implementation of the same algorithms).
    ///
    /// ```ignore
    /// // This example is not tested automatically because it doesn't
    /// // compile when the `rand_core_0_6` feature is disabled.
    /// use chacha8rand::ChaCha8Rand; // with rand_core_0_6 feature
    /// use rand::prelude::*; // rand version 0.8
    ///
    /// let mut rng = ChaCha8Rand::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456");
    /// if rng.gen_ratio(2, 3) {
    ///     println!("Nice weather we're having :)");
    /// } else {
    ///     println!("Awful weather, isn't it?");
    /// }
    /// let chan = rng.gen_range(1..100);
    /// let celsius = rng.gen_range(-5.0..=35.0);
    /// println!("Channel {chan} News said it'll be {celsius:.1} degrees tomorrow.");
    /// ```
    ///
    /// Taking the remainder modulo `n` to get an integer in `0..n` should be avoided because it
    /// introduces bias when `n` is not a power of two. This is easier to see when you try all the
    /// possibilities with a smaller number of bits than 32, e.g., with five bits and three options:
    ///
    /// ```
    /// let mut remainder_histogram = [0, 0, 0];
    /// for five_bit_number in 0..(1 << 5) {
    ///     remainder_histogram[five_bit_number % 3] += 1;
    /// }
    /// assert_eq!(remainder_histogram, [11, 11, 10]);
    /// ```
    ///
    /// In this example, the results 0 and 1 each have probability 11 / 32 = 34.375% and the result
    /// 2 has probability 10 / 32 = 31.25% instead of the desired 33.333..% for each.
    ///
    /// It may appear that the bias becomes very small when you use 32 bits instead of just five,
    /// but it can still cause problems at larger scales. Consider a scenario where you choose among
    /// `n = (u32::MAX / 3) * 2` (ca. 2.86 billion) items via `read_u32() % n`. For any given item,
    /// the odds of being chosen are already very small, with or without the bias. However, if you
    /// choose a few hundred items and look at them as a whole, you'll notice that roughly half of
    /// them are from the first *third* of the range `0..n`, and the other half are spread out
    /// across the rest of the range.
    ///
    /// More fully featured libraries like `rand` implement sampling algorithms that avoid this
    /// problem. They're also usually more efficient than computing the remainder, which is a
    /// relatively expensive operation even on modern CPUs.
    ///
    /// At other times, 32 bits is exactly what you need. For example, [tabulation
    /// hashing][tab-hash] with 32 bit output needs a table of random 32 bit integers. To hash a 64
    /// bit value this way, we could split it into 16 pieces of 4 bits each and use a table of 16 x
    /// 2^4 random `u32`s:
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// struct U64Hasher {
    ///     table: [[u32; 16]; 16],
    /// }
    ///
    /// impl U64Hasher {
    ///     fn new(rng: &mut ChaCha8Rand) -> Self {
    ///         let mut table = [[0; 16]; 16];
    ///         for row in &mut table {
    ///             for cell in row {
    ///                 *cell = rng.read_u32();
    ///             }
    ///         }
    ///         Self { table }
    ///     }
    ///
    ///     fn hash(&self, mut value: u64) -> u32 {
    ///         let mut hash = 0;
    ///         for row in &self.table {
    ///             hash ^= row[(value & 0xF) as usize];
    ///             value >>= 4;
    ///         }
    ///         debug_assert_eq!(value, 0);
    ///         hash
    ///     }
    /// }
    ///
    /// # let seed = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
    /// let mut rng = ChaCha8Rand::new(&seed);
    /// let hasher = U64Hasher::new(&mut rng);
    /// assert_ne!(hasher.hash(1 << 32), hasher.hash(1 << 36));
    /// ```
    ///
    /// [tab-hash]: https://en.wikipedia.org/wiki/Tabulation_hashing
    /// [rand-feature]: ./index.html#crate-features
    #[inline]
    pub fn read_u32(&mut self) -> u32 {
        const N: usize = size_of::<u32>();

        if self.bytes_consumed > BUF_OUTPUT_LEN - N {
            return self.read_u32_near_buffer_end();
        }
        let bytes = *array_ref![self.buf.output(), self.bytes_consumed, N];
        self.bytes_consumed += N;
        u32::from_le_bytes(bytes)
    }

    #[inline(never)]
    #[cold]
    fn read_u32_near_buffer_end(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.read_bytes(&mut buf);
        u32::from_le_bytes(buf)
    }

    /// Consume eight bytes of uniformly random data and return them as `u64`.
    ///
    /// This is always equivalent to [`ChaCha8Rand::read_bytes`] plus `u64::from_le_bytes`, but 99%
    /// of the time it's more efficient. If you simply need 64 or fewer uniformly random bits, this
    /// method enables this conveniently and without involving the `rand_*` crates.
    ///
    /// As discussed in the [the 32-bit variant][`ChaCha8Rand::read_u32`], you can and should use
    /// [`ChaCha8Rand`] with the rand crates for bounded integers in a range such as `0..n` or
    /// `m..=n`, to generate floating-point numbers and sample non-uniform distributions, to shuffle
    /// lists, and so on.
    ///
    /// # Examples
    ///
    /// With 64 bits, we can generate a 8x8 bitmap and render it as ASCII art. Clearly that's much
    /// better than a smaller (and non-square) bitmap with only 32 "pixels".
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// # let seed = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
    /// let mut rng = ChaCha8Rand::new(&seed);
    /// let mut bitmap = rng.read_u64();
    /// for _row in 0..8 {
    ///     for _column in 0..8 {
    ///         let pixel = ['X', '.'][(bitmap & 1) as usize];
    ///         print!("{pixel}");
    ///         bitmap >>= 1;
    ///     }
    ///     println!();
    /// }
    /// ```
    ///
    /// A more computer science-minded example would be [strongly universal hashing][univ-hash] of
    /// 32-bit integers into 32 or fewer bits. The strongly universal multiply-shift scheme by
    /// Dietzfelbinger needs two random, independent 64-bit parameters `a` and `b`:
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// # use std::num::NonZero;
    /// struct MulAddShift {
    ///     a: u64,
    ///     b: u64,
    /// }
    ///
    /// impl MulAddShift {
    ///     fn hash(&self, x: u32) -> u32 {
    ///         (u64::from(x).wrapping_mul(self.a).wrapping_add(self.b) >> 32) as u32
    ///     }
    /// }
    ///
    /// let mut rng = ChaCha8Rand::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456");
    /// let h = MulAddShift {
    ///     a: rng.read_u64(),
    ///     b: rng.read_u64()
    /// };
    /// // Truncating the outputs to two bits also gives a strongly universal family.
    /// // Strong universality implies uniformity - all hash values are equally likely.
    /// assert_eq!(h.hash(0) % 4, 2);
    /// assert_eq!(h.hash(1) % 4, 0);
    /// assert_eq!(h.hash(2) % 4, 3);
    /// assert_eq!(h.hash(3) % 4, 1);
    /// ```
    ///
    /// It's just a happy coincidence that we got every two-bit output exactly once in this small
    /// example, with this exact seed and these exact inputs. To be honest, I was a bit surprised
    /// that it worked out so perfectly.
    ///
    /// [univ-hash]: https://en.wikipedia.org/wiki/Universal_hashing
    #[inline]
    pub fn read_u64(&mut self) -> u64 {
        const N: usize = size_of::<u64>();
        // Same code as for u32. Making this code generic over `N` is more trouble than it's worth.
        if self.bytes_consumed > BUF_OUTPUT_LEN - N {
            return self.read_u64_near_buffer_end();
        }
        let bytes = *array_ref![self.buf.output(), self.bytes_consumed, N];
        self.bytes_consumed += N;
        u64::from_le_bytes(bytes)
    }

    #[inline(never)]
    #[cold]
    fn read_u64_near_buffer_end(&mut self) -> u64 {
        let mut buf = [0; 8];
        self.read_bytes(&mut buf);
        u64::from_le_bytes(buf)
    }

    /// Consume uniformly random bytes and write them into `dest`.
    ///
    /// This method is, in some sense, the most foundational way of using the generator. Other
    /// `read_*` methods behave as-if they read however many bytes they require, but they're more
    /// convenient and often more efficient than reading a small number of bytes manually.
    ///
    /// On the other hand, when you need a large chunk of randomness (hundreds of bytes or more),
    /// reading into a large buffer is very efficient because it boils down to one or several
    /// `memcpy`s from the generator's internal buffer. With a large enough buffer, this can produce
    /// several gigabytes per second with 128-bit SIMD, and the AVX2 backend goes roughly twice as
    /// fast. (For `read_u32` and `read_u64`, the difference is *much* more modest.)
    ///
    /// # Example
    ///
    /// You can use this to derive a new 32-byte seed for another [`ChaCha8Rand`] instance, but the
    /// [`ChaCha8Rand::read_seed`] helper makes this more convenient, so see examples there.
    ///
    /// Other use cases require more or fewer bytes. For example, random (v4) UUIDs are great for
    /// assigning arbitrary names to objects or events while avoiding collisions with other people
    /// (or their computers) doing the same thing. In most cases, you should generate a UUID
    /// directly from OS-provided entropy, which the [`uuid`][uuid] crate supports with
    /// `Uuid::new_v4()`. But in some cases it's more convenient to get a high-entropy seed from the
    /// OS at startup and feed it into a high-quality userspace RNG to create lots of UUIDs:
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// use uuid::Uuid;
    /// // For this use case, you *always* need a high-entropy seed!
    /// // A low-entropy seed (current time, chosen by humans, etc.), reusing a seed,
    /// // or cloning the generator leads to many colliding "UUIDs".
    /// let mut seed = [0; 32];
    /// getrandom::getrandom(&mut seed).expect("getrandom failure is 'highly unlikely'");
    /// let mut rng = ChaCha8Rand::new(&seed);
    /// let mut uuid_bytes = [0u8; 16];
    /// rng.read_bytes(&mut uuid_bytes);
    /// let uuid = Uuid::from_bytes(uuid_bytes);
    /// ```
    ///
    /// [uuid]: https://crates.io/crates/uuid
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

    /// Consume 32 uniformly random bytes, suitable for seeding another RNG instance.
    ///
    /// This is a simple wrapper around `read_bytes`, but returning an array by value is convenient
    /// when you want to use it as a seed. The ChaCha8Rand algorithm already replaces its seed with
    /// some of its earlier output after every iteration (992 bytes of output + 32 bytes of new
    /// seed). In this sense, it's not even that strange to use the other output in the same way.
    /// There's usually no point in manually re-seeding the *same* generator instance, but it's
    /// often useful to derive several independent generators from a "root" seed.
    ///
    /// Of course, the seed could also be used with any other PRNG algorithm that accepts 32-byte
    /// seeds. However, most generators that accept `[u8; 32]` as seed also use stream or block
    /// ciphers to generate large batches of data. There are some statistical generators that happen
    /// to have exactly 32 bytes of state, but these usually want 32- or 64-bit integers instead of
    /// raw bytes. In that case you might just call `read_u32` or `read_u64` a few times.
    ///
    /// # Examples
    ///
    /// In general, having multiple generator instances is useful when you want some *domain
    /// separation*. A Monte Carlo simulation that should be reproducible from a seed might also
    /// rely on some *Las Vegas* algorithms for auxillary tasks that don't affect the simulation's
    /// outcome (e.g., randomized quicksort or building a perfect hash table). If everything shares
    /// a single generator, any change to how much (or when) randomness is consumed will affect
    /// reproducibility of the simulation. On the other hand, debugging may be easier if the rest of
    /// the program also depends on the seed. In such cases, you could stretch a "root" seed into
    /// multiple seeds (without involving additional algorithms such as key derivation functions):
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// # fn quicksort_with_random_pivot<T, U>(_: &mut T, _: &mut U) {}
    /// let root_seed = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
    /// let mut root_rng = ChaCha8Rand::new(root_seed);
    /// let mut sim_rng = ChaCha8Rand::new(&root_rng.read_seed());
    /// let mut qsort_rng = ChaCha8Rand::new(&root_rng.read_seed());
    /// drop(root_rng); // no longer needed
    /// let steps = sim_rng.read_u32();
    /// # let steps = 3;
    /// for _ in 0..steps {
    ///     // ... generate some data using `sim_rng` ...
    ///     # let mut intermediate_results = [1, 2, 3];
    ///     quicksort_with_random_pivot(
    ///         &mut intermediate_results,
    ///         &mut qsort_rng,
    ///     );
    ///     // ... use the sorted data ...
    /// }
    /// ```
    ///
    /// Another example comes from roguelike games that use a single seed to drive procedural
    /// generation as well as the blow-by-blow game play. If one seeds leads to particularly
    /// interesting outcomes, players may want to share and reuse it. This falls flat if *any*
    /// change in how much randomness is consumed avalanches into entirely different outcomes down
    /// the line. For example, if parts of the map are generated on the fly, they might look
    /// entirely different depending on how many turns the player took to reach them. Instead, you
    /// can set up one generator for every aspect of the game's randomness, and derive seeds for all
    /// of them from the seed that the player deals with. Just make sure you don't [accidentally use
    /// the *same* seed][sts-corr-rand] for each of the generators.
    ///
    /// ```
    /// # use chacha8rand::ChaCha8Rand;
    /// struct GameStateGodObject {
    ///     map_rng: ChaCha8Rand,
    ///     encounter_rng: ChaCha8Rand,
    ///     ai_rng: ChaCha8Rand,
    ///     event_rng: ChaCha8Rand,
    ///     // ...
    /// }
    ///
    /// impl GameStateGodObject {
    ///     fn new(seed: [u8; 32]) -> Self {
    ///         let mut root = ChaCha8Rand::new(&seed);
    ///         let mut derive_rng = || ChaCha8Rand::new(&root.read_seed());
    ///         Self {
    ///             map_rng: derive_rng(),
    ///             encounter_rng: derive_rng(),
    ///             ai_rng: derive_rng(),
    ///             event_rng: derive_rng(),
    ///             // ...
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [sts-corr-rand]: https://forgottenarbiter.github.io/Correlated-Randomness/
    pub fn read_seed(&mut self) -> [u8; 32] {
        let mut seed = [0; 32];
        self.read_bytes(&mut seed);
        seed
    }

    /// Take a snapshot of the generator's current state.
    ///
    /// See [`ChaCha8State`] for more details and an example.
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

    /// Restore the generator's state from a snapshot taken before.
    ///
    /// See the documentation of [`ChaCha8State`] for more details and an example.
    ///
    /// # Errors
    ///
    /// This function never fails if `state` came from [`ChaCha8Rand::clone_state`] and was not
    /// modified. Otherwise (e.g., if you deserialize it from a file that someone fiddled with), it
    /// may fail because the `bytes_consumed` field is out of range. This field refers to a single
    /// iteration of ChaCha8Rand, which always produces 992 bytes of output. Thus, valid values are
    /// in the range `0..=992`.
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
#[allow(
    missing_docs,
    reason = "internal APIs only exposed unstably for benchmarks"
)]
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
