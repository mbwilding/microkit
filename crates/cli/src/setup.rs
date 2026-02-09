use crate::run_command;
use anyhow::{Context, Result};

pub fn setup() -> Result<()> {
    println!("Setting up environment");

    println!("Starting containers with podman-compose");
    run_command("podman-compose", &["up", "-d"])
        .context("Failed to start containers with podman-compose")?;

    println!("Initializing dapr");
    run_command("dapr", &["init", "--slim"]).context("Failed to initialize dapr")?;

    println!("Setup complete");
    Ok(())
}
