pub(crate) mod database;
pub(crate) mod new;
pub(crate) mod run;
pub(crate) mod setup;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use microkit::config::Config;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Parser)]
#[command(about = "MicroKit CLI tool to create and manage services", long_about = None)]
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
    Db(database::Commands),
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
        } => new::new(name, port_offset, description),
        Commands::Setup => setup::setup(),
        Commands::All => run::all(),
        Commands::Run { name } => run::binary(name),
        Commands::Db(cmd) => match cmd {
            database::Commands::Entity => database::entity(&config),
            database::Commands::Migrate { name } => database::migrate(&config, &name),
            database::Commands::Fresh => database::fresh(&config),
        },
    }
}

fn load_config() -> Result<Config> {
    let config_path = PathBuf::from("config.yml");
    let config_content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            // For supporting working within the MicroKit repository
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

pub(crate) fn run_command(program: &str, args: &[&str]) -> Result<()> {
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
