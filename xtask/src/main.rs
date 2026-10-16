use anyhow::{Result, ensure};
use std::process::Command;
fn run(args: &[&str]) -> Result<()> {
    ensure!(
        Command::new("cargo").args(args).status()?.success(),
        "cargo command failed: {args:?}"
    );
    Ok(())
}
fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("check") => {
            run(&["fmt", "--all", "--", "--check"])?;
            run(&[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])?;
            run(&["test", "--locked", "--workspace"])?;
            run(&["run", "--locked", "--bin", "contracts", "--", "--check"])
        }
        Some("test-api") => run(&["test", "--locked", "--test", "api"]),
        Some("test-process") => run(&["test", "--locked", "--test", "process"]),
        Some("contracts") => run(&["run", "--locked", "--bin", "contracts"]),
        _ => anyhow::bail!("use check, test-api, test-process, contracts"),
    }
}
