use anyhow::{Context, Result, bail};
use clap::Parser;
use microkit::config::Config;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

#[cfg(not(debug_assertions))]
use serde::Deserialize;
#[cfg(not(debug_assertions))]
use std::io::Cursor;
#[cfg(not(debug_assertions))]
use zip::ZipArchive;

#[derive(Parser)]
pub(crate) struct NewArgs {
    /// Name of the service
    name: String,
    /// Description of the service
    #[arg(short, long)]
    description: Option<String>,
    /// Port offset, this will offset your ports so you can run multiple services at the same time
    #[arg(short, long, default_value = "0")]
    port_offset: u16,
    /// The MicroKit git tag to create the service from (default: latest version from crates.io)
    #[arg(short, long)]
    tag: Option<String>,
}

#[cfg(not(debug_assertions))]
#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
}

#[cfg(not(debug_assertions))]
#[derive(Deserialize)]
struct CrateInfo {
    max_version: String,
}

pub async fn exec(args: NewArgs) -> Result<()> {
    exec_impl(args).await
}

#[cfg(debug_assertions)]
async fn exec_impl(args: NewArgs) -> Result<()> {
    println!("Creating new service '{}'", args.name);
    println!("[DEBUG MODE ACTIVE]");

    let test_dir = PathBuf::from(".test");
    if !test_dir.exists() {
        std::fs::create_dir(&test_dir).context("Failed to create .test directory")?;
    }
    let target_dir = test_dir.join(&args.name);

    if target_dir.exists() {
        bail!(
            "Cannot create service: directory '{}' already exists. Please choose a different name or remove the existing directory.",
            target_dir.display()
        );
    }

    std::fs::create_dir(&target_dir)
        .with_context(|| format!("Failed to create directory '{}'", target_dir.display()))?;

    println!("Debug mode: Using local template folder");
    copy_local_template(&target_dir).context("Failed to copy local template files")?;

    let cargo_disabled = target_dir.join("Cargo.toml-disabled");
    let cargo_toml = target_dir.join("Cargo.toml");
    if cargo_disabled.exists() {
        std::fs::rename(&cargo_disabled, &cargo_toml)
            .context("Failed to rename Cargo.toml-disabled to Cargo.toml")?;
    }

    update_config(&target_dir, &args.name, args.description, args.port_offset)?;

    fix_debug_cargo_paths(&target_dir)?;

    println!("Created service '{}' successfully", args.name);

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn exec_impl(args: NewArgs) -> Result<()> {
    println!("Creating new service '{}'", args.name);

    let target_dir = PathBuf::from(&args.name);

    if target_dir.exists() {
        bail!(
            "Cannot create service: directory '{}' already exists. Please choose a different name or remove the existing directory.",
            target_dir.display()
        );
    }

    std::fs::create_dir(&target_dir)
        .with_context(|| format!("Failed to create directory '{}'", target_dir.display()))?;

    let version = if let Some(tag) = args.tag {
        tag
    } else {
        let latest = get_latest_version().await?;
        println!("Using latest version: {}", latest);
        latest
    };

    get_template(&target_dir, &version)
        .await
        .context("Failed to extract template files")?;

    let cargo_disabled = target_dir.join("Cargo.toml-disabled");
    let cargo_toml = target_dir.join("Cargo.toml");
    if cargo_disabled.exists() {
        std::fs::rename(&cargo_disabled, &cargo_toml)
            .context("Failed to rename Cargo.toml-disabled to Cargo.toml")?;
    }

    update_config(&target_dir, &args.name, args.description, args.port_offset)?;

    // In release mode, update to specific version tag
    update_kit_reference(&target_dir, &version)?;

    println!("Created service '{}' successfully", args.name);

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn get_latest_version() -> Result<String> {
    println!("Fetching latest version from crates.io...");
    let url = "https://crates.io/api/v1/crates/microkit";

    let client = reqwest::Client::builder()
        .user_agent("microkit-cli")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to fetch version from crates.io")?;

    if !response.status().is_success() {
        bail!("Failed to fetch crates.io data: HTTP {}", response.status());
    }

    let data: CratesIoResponse = response
        .json()
        .await
        .context("Failed to parse crates.io response")?;

    Ok(data.crate_info.max_version)
}

#[cfg(not(debug_assertions))]
async fn get_template(target: &Path, tag: &str) -> Result<()> {
    println!("Downloading template from GitHub...");
    download_and_extract_template(target, tag)
        .await
        .context("Failed to download template from GitHub")?;

    Ok(())
}

#[cfg(debug_assertions)]
fn copy_local_template(target: &Path) -> Result<()> {
    let workspace_root = std::env::current_dir().context("Failed to get current directory")?;

    let template_dir = workspace_root.join("template");

    if !template_dir.exists() {
        bail!(
            "Template directory not found at '{}'. Make sure you're running from the workspace root.",
            template_dir.display()
        );
    }

    println!("Copying from local template: {}", template_dir.display());

    copy_dir_recursive(&template_dir, target).context("Failed to copy template directory")?;

    Ok(())
}

#[cfg(debug_assertions)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)
            .with_context(|| format!("Failed to create directory '{}'", dst.display()))?;
    }

    for entry in std::fs::read_dir(src)
        .with_context(|| format!("Failed to read directory '{}'", src.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy '{}' to '{}'",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn download_and_extract_template(target: &Path, tag: &str) -> Result<()> {
    let url = format!(
        "https://github.com/mbwilding/microkit/archive/refs/tags/{}.zip",
        tag
    );

    let client = reqwest::Client::builder()
        .user_agent("microkit-cli")
        .build()?;

    let response = client
        .get(&url)
        .send()
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

    // The prefix in the zip for tags: microkit-{version}/template/
    let prefix = format!("microkit-{}/template/", tag);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name();

        if !file_path.starts_with(&prefix) {
            continue;
        }

        let relative_path = file_path.strip_prefix(&prefix).unwrap();

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

#[cfg(not(debug_assertions))]
fn update_kit_reference(target_dir: &Path, tag: &str) -> Result<()> {
    let cargo_toml_path = target_dir.join("Cargo.toml");

    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)?;

    let mut doc = cargo_toml.parse::<DocumentMut>()?;

    if let Some(workspace) = doc["workspace"].as_table_mut()
        && let Some(deps) = workspace["dependencies"].as_table_mut()
        && deps.contains_key("microkit")
    {
        deps["microkit"] = toml_edit::value(tag);
    }

    std::fs::write(&cargo_toml_path, doc.to_string())?;

    Ok(())
}

#[cfg(debug_assertions)]
fn fix_debug_cargo_paths(target_dir: &Path) -> Result<()> {
    let cargo_toml_path = target_dir.join("Cargo.toml");

    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)?;

    let mut doc = cargo_toml.parse::<DocumentMut>()?;

    if let Some(workspace) = doc["workspace"].as_table_mut()
        && let Some(deps) = workspace["dependencies"].as_table_mut()
        && deps.contains_key("microkit")
    {
        let mut table = toml_edit::InlineTable::new();
        table.insert("path", "../../crates/microkit".into());
        deps["microkit"] = toml_edit::value(table);
    }

    std::fs::write(&cargo_toml_path, doc.to_string())?;

    Ok(())
}
