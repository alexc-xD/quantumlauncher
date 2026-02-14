use std::sync::mpsc::Sender;

use ql_core::{info, pt, GenericProgress, InstanceSelection, ListEntry, Loader, CLIENT};

use serde::Deserialize;

use super::InstancePackageError;

const DEFAULT_INSTANCE_URL: &str = "https://github.com/alexc-xD/dycraft-version/releases/download/releases/default-instance.json";

#[derive(Deserialize)]
pub struct DefaultInstanceConfig {
    pub name: String,
    pub version: String,
    pub version_type: String,
    pub loader: Option<LoaderConfig>,
}

#[derive(Deserialize)]
pub struct LoaderConfig {
    pub kind: String,           // "fabric", "forge", "quilt", etc.
    pub version: Option<String>, // None = latest
}

pub const OUT_OF: usize = 3;

/// Fetches instance configuration from a remote server and creates the instance.
///
/// # Errors
/// - Network request fails
/// - JSON parsing fails
/// - Instance creation fails
pub async fn fetch_and_create_default_instance(
    sender: Option<Sender<GenericProgress>>,
) -> Result<InstanceSelection, InstancePackageError> {
    fetch_and_create_from_url(DEFAULT_INSTANCE_URL, sender).await
}

/// Fetches instance configuration from a specified URL and creates the instance.
pub async fn fetch_and_create_from_url(
    url: &str,
    sender: Option<Sender<GenericProgress>>,
) -> Result<InstanceSelection, InstancePackageError> {
    if let Some(ref s) = sender {
        _ = s.send(GenericProgress {
            done: 0,
            total: OUT_OF,
            message: Some("Fetching instance config...".to_owned()),
            has_finished: false,
        });
    }

    pt!("Fetching default instance config from {url}");
    let response = CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| InstancePackageError::Request(e.into()))?;

    let config: DefaultInstanceConfig = response
        .json()
        .await
        .map_err(|e| InstancePackageError::Request(e.into()))?;

    create_instance_from_config(config, sender).await
}

async fn create_instance_from_config(
    config: DefaultInstanceConfig,
    sender: Option<Sender<GenericProgress>>,
) -> Result<InstanceSelection, InstancePackageError> {
    info!("Creating instance: {}", config.name);
    pt!("Version: {} ({})", config.version, config.version_type);

    if let Some(ref s) = sender {
        _ = s.send(GenericProgress {
            done: 1,
            total: OUT_OF,
            message: Some(format!("Creating instance '{}'...", config.name)),
            has_finished: false,
        });
    }

    let version = ListEntry::with_kind(config.version.clone(), &config.version_type);

    let (d_send, d_recv) = std::sync::mpsc::channel();
let sender_clone = sender.clone();
std::thread::spawn(move || {
    if let Some(s) = sender_clone {
        super::import::pipe_progress(d_recv, &s);
    } else {
        // drain the channel
        for _ in d_recv {}
    }
});

ql_instances::create_instance(config.name.clone(), version, Some(d_send), true).await?;

    let instance = InstanceSelection::new(&config.name, false);

    // Install loader if specified
    if let Some(loader) = config.loader {
        if let Some(ref s) = sender {
            _ = s.send(GenericProgress {
                done: 2,
                total: OUT_OF,
                message: Some(format!("Installing {} loader...", loader.kind)),
                has_finished: false,
            });
        }
let mod_type = match loader.kind.to_lowercase().as_str() {
    "fabric" => Loader::Fabric,
    "forge" => Loader::Forge,
    "quilt" => Loader::Quilt,
    "neoforge" => Loader::Neoforge,
    _ => Loader::Vanilla,
};

if !matches!(mod_type, Loader::Vanilla) {
            ql_mod_manager::loaders::install_specified_loader(
                instance.clone(),
                mod_type,
                sender.clone().map(std::sync::Arc::new),
                None,
            )
            .await
            .map_err(InstancePackageError::Loader)?;
        }
    }

    if let Some(ref s) = sender {
        _ = s.send(GenericProgress {
            done: OUT_OF,
            total: OUT_OF,
            message: Some("Done!".to_owned()),
            has_finished: true,
        });
    }

    info!("Finished creating default instance");
    Ok(instance)
}
