use crate::run_command;
use anyhow::{Context, Result};

pub fn all() -> Result<()> {
    println!("Running all services");
    run_command("dapr", &["run", "-f", "."]).context("Failed to run services with dapr")
}

pub fn binary(name: String) -> Result<()> {
    println!("Running binary: {}", &name);
    run_command("cargo", &["run", "--bin", &name])
        .with_context(|| format!("Failed to run binary '{}'", &name))
}
