use xflags::xflags;
use xshell::{cmd, Shell};

xflags! {
    cmd xtask {
        cmd crosstest {}
    }
}

fn main() -> xshell::Result<()> {
    match Xtask::from_env_or_exit().subcommand {
        XtaskCmd::Crosstest(Crosstest {}) => crosstest(),
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
