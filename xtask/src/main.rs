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

fn crosstest() -> xshell::Result<()> {
    let sh = Shell::new()?;
    let targets = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"];
    for target in targets {
        // Testing the x86_64 target on an x86_64 host means rustflags from $CARGO_HOME/.config.toml
        // are picked up within the container. This is a problem if those contain
        // -Clink-arg=-fuse-lld, which doesn't work inside the container. Setting the RUSTFLAGS
        // variable overrides the flags from the config files.
        cmd!(sh, "cross test --target {target} --all-targets")
            .env("RUSTFLAGS", "")
            .run()?;
    }
    Ok(())
}

fn bench_in_wasmtime() -> xshell::Result<()> {
    let sh = Shell::new()?;
    cmd!(
        sh,
        "cargo build --release --target wasm32-wasip1 --bin bench"
    )
    .env("RUSTFLAGS", "-Ctarget-feature=+simd128")
    .run()?;
    cmd!(sh, "wasmtime run target/wasm32-wasip1/release/bench.wasm").run()?;
    Ok(())
}
