use xflags::xflags;
use xshell::{cmd, Shell};

xflags! {
    cmd xtask {
        cmd testmatrix {}
        cmd wasmbench {}
    }
}

fn main() -> xshell::Result<()> {
    match Xtask::from_env_or_exit().subcommand {
        XtaskCmd::Testmatrix(Testmatrix {}) => test_matrix(),
        XtaskCmd::Wasmbench(Wasmbench {}) => bench_in_wasmtime(),
    }
}

const WASM_RUNNER_KEY: &str = "CARGO_TARGET_WASM32_WASIP1_RUNNER";
const WASM_RUNNER_VAL: &str = "cargo bin wasmtime";

fn test_matrix() -> xshell::Result<()> {
    let sh = Shell::new()?;

    // Any combination of features should work and run tests.
    cmd!(sh, "cargo hack test -p chacha8rand --feature-powerset").run()?;

    let targets = [
        "aarch64-unknown-linux-gnu", // for neon
        "i586-unknown-linux-gnu",    // for x86 without sse2 statically enabled
        "i686-unknown-linux-gnu",    // for 32-bit compatibility in sse2 and avx2 modules
        "s390x-unknown-linux-gnu",   // for big endian
        "x86_64-unknown-linux-gnu",  // for sse2 and avx2
    ];
    for target in targets {
        // Testing the x86_64 target on an x86_64 host means rustflags from $CARGO_HOME/.config.toml
        // are picked up within the container. This is a problem if those contain
        // -Clink-arg=-fuse-lld, which doesn't work inside the container. Setting the RUSTFLAGS
        // variable overrides the flags from the config files.
        cmd!(
            sh,
            "cargo bin cross test --target {target} --all-targets --all-features"
        )
        .env("RUSTFLAGS", "")
        .run()?;
    }
    // x86_64-unknown-none is an x86 target without std, so it can't *run* the tests but it's useful
    // as a smoke test for no_std support, especially w.r.t. the use of std for feature detection in
    // the avx2 backend.
    cmd!(
        sh,
        "cargo check --target x86_64-unknown-none -p chacha8rand"
    )
    .run()?;

    // Test wasm with and without simd128
    for flags in ["", "-Ctarget-feature=+simd128"] {
        cmd!(sh, "cargo test --target wasm32-wasip1")
            .env(WASM_RUNNER_KEY, WASM_RUNNER_VAL)
            .env("RUSTFLAGS", flags)
            .run()?;
    }
    Ok(())
}

fn bench_in_wasmtime() -> xshell::Result<()> {
    let sh = Shell::new()?;
    cmd!(
        sh,
        "cargo run --release --target wasm32-wasip1 -p benchmarks"
    )
    .env("RUSTFLAGS", "-Ctarget-feature=+simd128")
    .env(WASM_RUNNER_KEY, WASM_RUNNER_VAL)
    .run()?;
    Ok(())
}
