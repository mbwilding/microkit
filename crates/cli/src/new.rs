use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use microkit::config::Config;
use std::path::{Path, PathBuf};

static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../template");

pub fn new(name: String, port_offset: u16, description: Option<String>) -> Result<()> {
    println!("Creating new service '{}'", name);

    let target_dir = PathBuf::from(&name);
    if target_dir.exists() {
        bail!("Directory '{}' already exists", name);
    }

    std::fs::create_dir(&target_dir)
        .with_context(|| format!("Failed to create directory '{}'", name))?;

    extract_dir(&TEMPLATE_DIR, &target_dir).context("Failed to extract template files")?;

    let cargo_disabled = target_dir.join("Cargo.toml-disabled");
    let cargo_toml = target_dir.join("Cargo.toml");
    if cargo_disabled.exists() {
        std::fs::rename(&cargo_disabled, &cargo_toml)
            .context("Failed to rename Cargo.toml-disabled to Cargo.toml")?;
    }

    update_config(&target_dir, &name, description, port_offset)?;

    println!("Created service '{}' successfully", name);

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
    target_dir: &Path,
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
