use crate::run_command;
use anyhow::{Context, Result};

pub fn exec() -> Result<()> {
    println!("Setting up environment");

    println!("Starting containers");
    run_command("podman-compose", &["up", "-d"])
        .context("Failed to start containers with podman-compose")?;

    println!("Initializing dapr");
    let _ = run_command("dapr", &["uninstall"]);
    run_command("dapr", &["init", "--slim"]).context("Failed to initialize dapr")?;

    println!("Setup complete");
    Ok(())
}
