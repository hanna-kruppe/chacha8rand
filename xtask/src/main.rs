use xflags::xflags;
use xshell::{Shell, cmd};

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
    // ... and also be warning-free
    cmd!(
        sh,
        "cargo hack clippy -p chacha8rand --feature-powerset -- --deny warnings"
    )
    .run()?;

    let cross_targets = [
        "aarch64-unknown-linux-gnu", // for neon
        "i586-unknown-linux-gnu",    // for x86 without sse2 statically enabled
        "i686-unknown-linux-gnu",    // for 32-bit compatibility in sse2 and avx2 modules
        "s390x-unknown-linux-gnu",   // for big endian
        "x86_64-unknown-linux-gnu",  // for sse2 and avx2
    ];
    for target in cross_targets {
        // Run clippy for each target to catch issues in cfg'd out code, with -Dwarnings so they
        // won't just be drowned in a sea of output.
        cmd!(
            sh,
            "cargo clippy --target {target} -p chacha8rand --all-features -- --deny warnings"
        )
        .run()?;

        // Run tests both with and without crate features to exercise static vs. dynamic feature
        // detection.
        for feat in ["--no-default-features", "--all-features"] {
            // Overriding RUSTFLAGS for `cross test` prevents the container picking up RUSTFLAGS
            // meant for the host (e.g., from $CARGO_HOME/config.toml) which can break stuff.
            cmd!(
                sh,
                "cargo bin cross test --target {target} --all-targets {feat}"
            )
            .env("RUSTFLAGS", "")
            .run()?;
        }
    }
    // x86_64-unknown-none is an x86 target without std, so it can't *run* the tests but it's useful
    // as a smoke test for no_std support, especially w.r.t. the use of std for feature detection in
    // the avx2 backend.
    cmd!(
        sh,
        "cargo clippy --target x86_64-unknown-none -p chacha8rand -- --deny warnings"
    )
    .run()?;

    // Test wasm with and without simd128
    for flags in ["", "-Ctarget-feature=+simd128"] {
        cmd!(sh, "cargo test --target wasm32-wasip1")
            .env(WASM_RUNNER_KEY, WASM_RUNNER_VAL)
            .env("RUSTFLAGS", flags)
            .run()?;
        // What's that? You guessed it: more clippy!
        cmd!(sh, "cargo clippy --target wasm32-wasip1 -- --deny warnings").run()?;
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
