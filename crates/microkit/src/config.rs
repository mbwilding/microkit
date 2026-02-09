use anyhow::{Context, Result};
use serde::Deserialize;

#[cfg(feature = "auth")]
use crate::auth::AuthConfig;

pub async fn get() -> Result<Config> {
    let file = "config.yml";
    let contents = tokio::fs::read_to_string(&file).await.context(format!(
        "Could not find '{}' in current working directory",
        &file
    ))?;
    let config =
        serde_yaml_ng::from_str(&contents).context(format!("Could not deserialize '{}'", &file))?;
    Ok(config)
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub service_name: String,
    pub service_desc: Option<String>,
    pub host: Option<String>,
    pub log_level: Option<String>,
    pub port_offset: Option<u16>,
    #[cfg(feature = "database")]
    pub database_url: Option<String>,
    #[cfg(feature = "database")]
    pub database_name: Option<String>,
    #[cfg(feature = "database")]
    pub database_drop: Option<bool>,
    #[cfg(feature = "auth")]
    pub auth: Option<AuthConfigYaml>,
    #[cfg(feature = "otel")]
    pub otel: Option<OtelConfig>,
}

impl Config {
    /// Create an AuthConfig from the configuration
    #[cfg(feature = "auth")]
    pub fn create_auth_config(&self) -> Result<Option<AuthConfig>> {
        let Some(auth_config) = &self.auth else {
            return Ok(None);
        };

        let mut auth = AuthConfig::oidc(auth_config.issuer.clone(), auth_config.jwks_uri.clone());

        if let Some(audience) = &auth_config.audience {
            auth = auth.with_audience(audience.clone());
        }

        if let Some(client_secret) = &auth_config.client_secret {
            auth = auth.with_client_secret(client_secret.clone());
        }

        Ok(Some(auth))
    }
}

#[cfg(feature = "otel")]
#[derive(Debug, Deserialize, Clone)]
pub struct OtelConfig {
    pub url: String,
    pub token: String,
}

/// Authentication configuration from YAML
#[cfg(feature = "auth")]
#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfigYaml {
    /// OIDC issuer URL
    /// For Cognito: https://cognito-idp.{region}.amazonaws.com/{userPoolId}
    pub issuer: String,
    /// OIDC JWKS URI
    /// For Cognito: https://cognito-idp.{region}.amazonaws.com/{userPoolId}/.well-known/jwks.json
    pub jwks_uri: String,
    /// Expected audience/client ID
    pub audience: Option<String>,
    /// Documentor: Default scopes
    pub scopes: Option<Vec<String>>,
    /// Documentor: Client ID
    pub client_id: Option<String>,
    /// Documentor: Client secret (Provide within config-private.yml so it doesn't get committed)
    pub client_secret: Option<String>,
}
