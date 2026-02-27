use anyhow::{Context, Result, anyhow};
use axum::{
    RequestPartsExt,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// JWT claims from OIDC token
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JwtClaims {
    /// Subject - unique user identifier
    pub sub: String,
    /// Email address
    pub email: Option<String>,
    // TODO: Handle having only ``groups` and cognito:groups will populate on fallback
    /// Cognito user groups
    #[serde(rename = "cognito:groups")]
    pub cognito_groups: Option<Vec<String>>,
    /// Generic groups field (for non-Cognito OIDC providers)
    pub groups: Option<Vec<String>>,
    /// Token expiration time (Unix timestamp)
    pub exp: usize,
    /// Issuer URL
    pub iss: String,
    /// Token issue time (Unix timestamp)
    pub iat: Option<usize>,
    /// Audience (client ID)
    pub aud: Option<serde_json::Value>,
}

/// Authenticated user extracted from validated JWT
///
/// Add this as a parameter to any handler that requires authentication
#[derive(Debug, Clone, Default)]
pub struct AuthenticatedUser {
    /// Sub claim
    pub sub: String,
    /// User email address
    pub email: Option<String>,
    /// User groups/roles
    pub groups: Vec<String>,
    /// Raw JWT claims
    pub claims: JwtClaims,
}

impl AuthenticatedUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.groups.iter().any(|g| g == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
}

/// Auth configuration for OIDC
#[derive(Clone)]
pub struct AuthConfig {
    jwks_uri: String,
    issuer: String,
    audience: Option<String>,
    /// Cached JWKS keys
    jwks_cache: Arc<RwLock<Option<JwkSet>>>,
    /// Optional client secret for API key authentication
    client_secret: Option<String>,
}

impl AuthConfig {
    /// Create auth config for generic OIDC provider
    pub fn oidc(issuer: String, jwks_uri: String) -> Self {
        Self {
            jwks_uri,
            issuer,
            audience: None,
            jwks_cache: Arc::new(RwLock::new(None)),
            client_secret: None,
        }
    }

    /// Set expected audience (client ID) for token validation
    pub fn with_audience(mut self, audience: String) -> Self {
        self.audience = Some(audience);
        self
    }

    /// Set client secret
    pub fn with_client_secret(mut self, client_secret: String) -> Self {
        self.client_secret = Some(client_secret);
        self
    }

    /// Validate JWT token
    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims> {
        let header = decode_header(token).context("Failed to decode JWT header")?;

        let kid = header
            .kid
            .ok_or_else(|| anyhow!("JWT missing 'kid' in header"))?;

        let key = self.get_decoding_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);

        if let Some(aud) = &self.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        let token_data =
            decode::<JwtClaims>(token, &key, &validation).context("Failed to validate JWT")?;

        Ok(token_data.claims)
    }

    /// Get decoding key for a specific key ID
    async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey> {
        {
            let cache = self.jwks_cache.read().await;
            if let Some(jwks) = cache.as_ref() {
                return self.find_key_in_jwks(jwks, kid);
            }
        }

        let jwks = self.fetch_jwks().await?;

        let mut cache = self.jwks_cache.write().await;
        *cache = Some(jwks.clone());

        self.find_key_in_jwks(&jwks, kid)
    }

    /// Fetch JWKS from the configured endpoint
    async fn fetch_jwks(&self) -> Result<JwkSet> {
        let response = reqwest::get(&self.jwks_uri)
            .await
            .context("Failed to fetch JWKS")?;

        let jwks: JwkSet = response.json().await.context("Failed to parse JWKS JSON")?;

        Ok(jwks)
    }

    /// Find a specific key in the JWKS
    fn find_key_in_jwks(&self, jwks: &JwkSet, kid: &str) -> Result<DecodingKey> {
        let jwk = jwks
            .find(kid)
            .ok_or_else(|| anyhow!("Key '{}' not found in JWKS", kid))?;

        DecodingKey::from_jwk(jwk).context("Failed to create decoding key from JWK")
    }

    /// Manually refresh the JWKS cache
    pub async fn refresh_jwks(&self) -> Result<()> {
        let jwks = self.fetch_jwks().await?;
        let mut cache = self.jwks_cache.write().await;
        *cache = Some(jwks);
        Ok(())
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Missing or invalid Authorization header".to_string(),
                )
            })?;

        let auth_config = parts
            .extensions
            .get::<AuthConfig>()
            .ok_or_else(|| {
                tracing::error!(
                    "AuthConfig not found in request extensions. \
                         Did you forget to add it via middleware or state?"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authentication not configured".to_string(),
                )
            })?
            .clone();

        // Validate JWT token
        let claims = auth_config
            .validate_token(bearer.token())
            .await
            .map_err(|e| {
                tracing::warn!("JWT validation failed: {}", e);
                (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e))
            })?;

        let groups = claims
            .cognito_groups
            .clone()
            .or_else(|| claims.groups.clone())
            .unwrap_or_default();

        Ok(AuthenticatedUser {
            sub: claims.sub.clone(),
            email: claims.email.clone(),
            groups,
            claims,
        })
    }
}

pub async fn inject_auth_config(
    axum::extract::State(config): axum::extract::State<AuthConfig>,
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    request.extensions_mut().insert(config);
    next.run(request).await
}
