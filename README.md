# ChaCha8Rand Implementation in Rust

Reproducible, robust and (last but not least) fast pseudorandomness.

This crate implements the [chacha8rand][spec] specification, originally designed
for Go's `math/rand/v2` package. The language-independent specification and test
vector helps with long-term reproducibility and interoperability. Building on
the ChaCha8 stream cipher ensures high statistical quality and removes entire
classes of "you're holding it wrong"-style problems that lead to sub-par output.
It's also carefully designed and implemented (using SIMD instructions when
available) to be so fast that it shouldn't ever be a bottleneck. However, it
should not be used for cryptography.

See the [documentation][docsrs] for more details.

Dual-licensed under Apache 2.0 or MIT at your option.

[spec]: https://c2sp.org/chacha8rand
[docsrs]: https://docs.rs/chacha8rand/
