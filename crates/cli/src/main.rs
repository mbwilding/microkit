use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use include_dir::{Dir, include_dir};
use microkit::config::Config;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../template");

#[derive(Parser)]
#[command(about = "A CLI tool to manage services", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates a new service
    New {
        /// Name of the service
        name: String,
        /// Port offset, this will offset your ports so you can run multiple services at the same time
        port_offset: u16,
        /// Description of the service
        description: Option<String>,
    },
    /// Setup the environment
    Setup,
    /// Run all services using dapr
    All,
    /// Run a specific binary
    Run {
        /// Name of the binary to run
        name: String,
    },
    /// Database commands
    #[command(subcommand)]
    Db(DbCommands),
}

#[derive(Subcommand)]
enum DbCommands {
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

fn load_config() -> Result<Config> {
    let config_path = PathBuf::from("config.yml");
    let config_content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            // For supporting working within the microkit root
            let template_dir = PathBuf::from("template");
            let template_config_path = template_dir.join("config.yml");
            match std::fs::read_to_string(&template_config_path) {
                Ok(content) => {
                    let _ = std::env::set_current_dir(&template_dir);
                    content
                }
                Err(_) => return Err(e.into()),
            }
        }
    };

    let config: Config =
        serde_yaml_ng::from_str(&config_content).context("Failed to parse config.yml")?;

    Ok(config)
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let cmd_str = format!("{} {}", program, args.join(" "));

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd_str))?;

    let child_id = child.id();
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();

    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::SeqCst);

        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            let _ = kill(Pid::from_raw(-(child_id as i32)), Signal::SIGINT);
            let _ = kill(Pid::from_raw(child_id as i32), Signal::SIGINT);
        }

        #[cfg(windows)]
        {
            use windows::Win32::System::Console::CTRL_C_EVENT;
            use windows::Win32::System::Console::GenerateConsoleCtrlEvent;
            let _ = unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, child_id) };
        }
    })
    .context("Failed to set Ctrl+C handler")?;

    let output = child.wait()?;

    if !output.success() {
        if interrupted.load(Ordering::SeqCst) {
            return Ok(());
        }
        anyhow::bail!("Exit {}: {}", output, cmd_str);
    }

    Ok(())
}

fn new(name: String, port_offset: u16, description: Option<String>) -> Result<()> {
    println!("Creating new service '{}'", name);

    let target_dir = PathBuf::from(&name);
    if target_dir.exists() {
        bail!("Directory '{}' already exists", name);
    }

    std::fs::create_dir(&target_dir)
        .with_context(|| format!("Failed to create directory '{}'", name))?;

    // Extract all files from the embedded template
    extract_dir(&TEMPLATE_DIR, &target_dir).context("Failed to extract template files")?;

    // Rename Cargo.toml-disabled to Cargo.toml
    let cargo_disabled = target_dir.join("Cargo.toml-disabled");
    let cargo_toml = target_dir.join("Cargo.toml");
    if cargo_disabled.exists() {
        std::fs::rename(&cargo_disabled, &cargo_toml)
            .context("Failed to rename Cargo.toml-disabled to Cargo.toml")?;
    }

    // Update config.yml with the provided name, description, and port_offset
    update_config(&target_dir, &name, description, port_offset)?;

    println!("Created service '{}' successfully", name);
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  mk setup");

    Ok(())
}

fn extract_dir(dir: &Dir, target: &PathBuf) -> Result<()> {
    for file in dir.files() {
        let file_path = target.join(file.path());

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory '{}'", parent.display()))?;
        }

        std::fs::write(&file_path, file.contents())
            .with_context(|| format!("Failed to write file '{}'", file_path.display()))?;
    }

    for subdir in dir.dirs() {
        let subdir_path = target.join(subdir.path());
        std::fs::create_dir_all(&subdir_path)
            .with_context(|| format!("Failed to create directory '{}'", subdir_path.display()))?;
        extract_dir(subdir, target)?;
    }

    Ok(())
}

fn update_config(
    target_dir: &PathBuf,
    name: &str,
    description: Option<String>,
    port_offset: u16,
) -> Result<()> {
    let config_path = target_dir.join("config.yml");
    let config_content =
        std::fs::read_to_string(&config_path).context("Failed to read config.yml")?;

    let mut config: Config =
        serde_yaml_ng::from_str(&config_content).context("Failed to parse config.yml")?;

    config.service_name = name.to_string();
    config.service_desc = description;
    config.port_offset = Some(port_offset);

    let updated_content =
        serde_yaml_ng::to_string(&config).context("Failed to serialize config.yml")?;
    std::fs::write(&config_path, updated_content).context("Failed to write updated config.yml")?;

    Ok(())
}

fn setup() -> Result<()> {
    println!("Setting up environment");

    println!("Starting containers with podman-compose");
    run_command("podman-compose", &["up", "-d"])
        .context("Failed to start containers with podman-compose")?;

    println!("Initializing dapr");
    run_command("dapr", &["init", "--slim"]).context("Failed to initialize dapr")?;

    println!("Setup complete");
    Ok(())
}

fn run_all() -> Result<()> {
    println!("Running all services");
    run_command("dapr", &["run", "-f", "."]).context("Failed to run services with dapr")
}

fn run_binary(name: String) -> Result<()> {
    println!("Running binary: {}", &name);
    run_command("cargo", &["run", "--bin", &name])
        .with_context(|| format!("Failed to run binary '{}'", &name))
}

fn db_entity(config: &Config) -> Result<()> {
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

fn db_migrate(config: &Config, name: &str) -> Result<()> {
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

fn db_fresh(config: &Config) -> Result<()> {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = load_config().context(
        "Ensure your current working directory is in a service and it contains a valid config.yml",
    )?;

    match cli.command {
        Commands::New {
            name,
            port_offset,
            description,
        } => new(name, port_offset, description),
        Commands::Setup => setup(),
        Commands::All => run_all(),
        Commands::Run { name } => run_binary(name),
        Commands::Db(cmd) => match cmd {
            DbCommands::Entity => db_entity(&config),
            DbCommands::Migrate { name } => db_migrate(&config, &name),
            DbCommands::Fresh => db_fresh(&config),
        },
    }
}
