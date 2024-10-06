use xflags::xflags;
use xshell::{cmd, Shell};

xflags! {
    cmd xtask {
        cmd crosstest {}
        cmd wasmbench {}
    }
}

fn main() -> xshell::Result<()> {
    match Xtask::from_env_or_exit().subcommand {
        XtaskCmd::Crosstest(Crosstest {}) => crosstest(),
        XtaskCmd::Wasmbench(Wasmbench {}) => bench_in_wasmtime(),
    }
}

const WASM_RUNNER_ENV: &str = "CARGO_TARGET_WASM32_WASIP1_RUNNER";

fn crosstest() -> xshell::Result<()> {
    let sh = Shell::new()?;
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
            "cross test --target {target} --all-targets --all-features"
        )
        .env("RUSTFLAGS", "")
        .run()?;
    }
    cmd!(sh, "cargo test --target wasm32-wasip1")
        .env(WASM_RUNNER_ENV, "wasmtime")
        .run()?;
    Ok(())
}

fn bench_in_wasmtime() -> xshell::Result<()> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo run --release --target wasm32-wasip1 --bin bench")
        .env("RUSTFLAGS", "-Ctarget-feature=+simd128")
        .env(WASM_RUNNER_ENV, "wasmtime")
        .run()?;
    Ok(())
}
