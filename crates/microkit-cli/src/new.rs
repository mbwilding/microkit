use anyhow::{Context, Result, bail};
use clap::Parser;
use microkit::config::Config;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

#[derive(Parser)]
pub(crate) struct NewArgs {
    /// Name of the service
    #[arg(short, long)]
    name: String,
    /// Description of the service
    #[arg(short, long)]
    description: Option<String>,
    /// Port offset, this will offset your ports so you can run multiple services at the same time
    #[arg(short, long)]
    port_offset: u16,
    /// The MicroKit git branch to create the service from (default: main)
    #[arg(short, long)]
    branch: Option<String>,
}

pub async fn new(args: NewArgs) -> Result<()> {
    println!("Creating new service '{}'", args.name);

    let target_dir = PathBuf::from(&args.name);
    if target_dir.exists() {
        bail!(
            "Cannot create service: directory '{}' already exists. Please choose a different name or remove the existing directory.",
            args.name
        );
    }

    std::fs::create_dir(&target_dir)
        .with_context(|| format!("Failed to create directory '{}'", args.name))?;

    get_template(&target_dir, args.branch)
        .await
        .context("Failed to extract template files")?;

    let cargo_disabled = target_dir.join("Cargo.toml-disabled");
    let cargo_toml = target_dir.join("Cargo.toml");
    if cargo_disabled.exists() {
        std::fs::rename(&cargo_disabled, &cargo_toml)
            .context("Failed to rename Cargo.toml-disabled to Cargo.toml")?;
    }

    update_config(&target_dir, &args.name, args.description, args.port_offset)?;
    update_kit_reference(&target_dir)?;

    println!("Created service '{}' successfully", args.name);

    Ok(())
}

#[allow(unused_variables)]
async fn get_template(target: &Path, branch: Option<String>) -> Result<()> {
    #[cfg(debug_assertions)]
    {
        let template_path = PathBuf::from("template");
        if !template_path.exists() {
            bail!(
                "Template directory not found at: {}",
                template_path.display()
            );
        }
        println!("Using local template");
        copy_dir_all(&template_path, target).context("Failed to copy local template")?;
    }

    #[cfg(not(debug_assertions))]
    {
        // Release build: download from GitHub
        println!("Downloading latest template from GitHub...");
        download_and_extract_template(target, branch)
            .await
            .context("Failed to download template from GitHub")?;
    }

    Ok(())
}

#[cfg(debug_assertions)]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory: {}", dst.display()))?;

    for entry in std::fs::read_dir(src)
        .with_context(|| format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        // Skip any directories that look like generated projects
        if path.is_dir() {
            let name_str = file_name.to_string_lossy();
            // Skip hidden directories except .cargo
            if name_str.starts_with('.') && name_str != ".cargo" {
                continue;
            }
            copy_dir_all(&path, &dest_path)
                .with_context(|| format!("Failed to copy directory: {}", path.display()))?;
        } else {
            std::fs::copy(&path, &dest_path).with_context(|| {
                format!(
                    "Failed to copy file: {} to {}",
                    path.display(),
                    dest_path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn download_and_extract_template(target: &Path, branch: Option<String>) -> Result<()> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let url = format!(
        "https://github.com/mbwilding/microkit/archive/refs/heads/{}.zip",
        branch.as_deref().unwrap_or("main")
    );
    let response = reqwest::get(url)
        .await
        .context("Failed to download template from GitHub")?;

    if !response.status().is_success() {
        bail!("Failed to download template: HTTP {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read template download")?;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).context("Failed to read zip archive")?;

    let prefix = "microkit-main/template/";

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name();

        // Only extract files from the template directory
        if !file_path.starts_with(prefix) {
            continue;
        }

        // Remove the prefix to get the relative path
        let relative_path = file_path.strip_prefix(prefix).unwrap();

        if relative_path.is_empty() {
            continue;
        }

        let out_path = target.join(relative_path);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

fn update_config(
    target_dir: &Path,
    name: &str,
    description: Option<String>,
    port_offset: u16,
) -> Result<()> {
    let config_path = target_dir.join("microkit.yml");
    let config_content =
        std::fs::read_to_string(&config_path).context("Failed to read microkit.yml")?;

    let mut config: Config =
        serde_yaml_ng::from_str(&config_content).context("Failed to parse microkit.yml")?;

    config.service_name = name.to_string();
    config.service_desc = description;
    config.port_offset = Some(port_offset);

    let updated_content =
        serde_yaml_ng::to_string(&config).context("Failed to serialize microkit.yml")?;
    std::fs::write(&config_path, updated_content)
        .context("Failed to write updated microkit.yml")?;

    Ok(())
}

fn update_kit_reference(target_dir: &Path) -> Result<()> {
    let cargo_toml_path = target_dir.join("Cargo.toml");

    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)?;

    let mut doc = cargo_toml.parse::<DocumentMut>()?;

    if let Some(workspace) = doc["workspace"].as_table_mut()
        && let Some(deps) = workspace["dependencies"].as_table_mut()
        && deps.contains_key("microkit")
    {
        let version = env!("CARGO_PKG_VERSION");

        if version != "0.0.0" {
            deps["microkit"]["version"] = toml_edit::value(version);
        } else {
            deps["microkit"]["path"] =
                toml_edit::value(format!("../{}", deps["microkit"]["path"].as_str().unwrap()));
        }
    }

    std::fs::write(&cargo_toml_path, doc.to_string())?;

    Ok(())
}
