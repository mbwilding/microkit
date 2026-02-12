use crate::run_command;
use anyhow::{Context, Result};

pub fn exec(name: Option<String>) -> Result<()> {
    if let Some(name) = name {
        println!("Running binary: {}", &name);
        run_command("cargo", &["run", "--bin", &name])
            .with_context(|| format!("Failed to run binary '{}'", &name))
    } else {
        println!("Running all services");
        run_command("dapr", &["run", "-f", "."]).context("Failed to run services with dapr")
    }
}
