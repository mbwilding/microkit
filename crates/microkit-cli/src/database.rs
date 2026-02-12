use crate::run_command;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use microkit::config::Config;

#[derive(Subcommand)]
pub enum Commands {
    /// Generate entity files from database
    Entity,
    /// Generate a new migration
    Migrate {
        /// Name of the migration
        name: String,
    },
    /// Drop all tables and re-apply all migrations
    Fresh,
}

pub fn entity(config: &Config) -> Result<()> {
    println!("Generating entities");
    let (_database_url, database_name, database_with_name) = get_database_details(config)?;
    run_command(
        "sea-orm-cli",
        &[
            "generate",
            "entity",
            "--database-url",
            &database_with_name,
            "--database-schema",
            database_name,
            // "--with-prelude",
            // "all",
            "--with-serde",
            "both",
            "--output-dir",
            "crates/entities/src",
        ],
    )
    .context("Failed to generate entity files from database")
}

pub fn migrate(config: &Config, name: &str) -> Result<()> {
    println!("Generating migration: {}", name);
    let (database_url, database_name, _database_with_name) = get_database_details(config)?;
    run_command(
        "sea-orm-cli",
        &[
            "migrate",
            "generate",
            "-d",
            "crates/migrations",
            "--local-time",
            "--database-url",
            database_url,
            "--database-schema",
            database_name,
            name,
        ],
    )
    .with_context(|| format!("Failed to generate migration '{}'", name))
}

pub fn fresh(config: &Config) -> Result<()> {
    println!("Dropping all tables and re-applying migrations");
    let (database_url, database_name, _database_with_name) = get_database_details(config)?;
    run_command(
        "sea-orm-cli",
        &[
            "migrate",
            "fresh",
            "-d",
            "crates/migrations",
            "--database-url",
            database_url,
            "--database-schema",
            database_name,
        ],
    )
    .context("Failed to refresh database migrations")
}

fn get_database_details(config: &Config) -> Result<(&str, &str, String)> {
    let database_url = match &config.database_url {
        Some(x) => x,
        None => bail!("database_url missing from config"),
    };

    let database_name = match &config.database_name {
        Some(x) => x,
        None => bail!("database_name missing from config"),
    };

    let database_with_name = format!("{database_url}/{database_name}");

    Ok((database_url, database_name, database_with_name))
}
