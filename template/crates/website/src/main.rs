use api::endpoints::api::users::{UserRequest, UserResponse};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// microkit.yml is two levels up from this crate's manifest dir
const MICROKIT_YML: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../microkit.yml");

const API_BASE: &str = "http://localhost:50000";
const CALLBACK_PORT: u16 = 4444;
const CALLBACK_URI: &str = "http://localhost:4444/callback";

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// ---------------------------------------------------------------------------
// Config — mirrors AuthConfigYaml in microkit/src/config.rs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AppConfig {
    auth: Option<AuthConfigYaml>,
}

/// Mirrors `AuthConfigYaml` from the microkit crate exactly.
/// The server-side fields (jwks_uri, audience, client_secret) are parsed but unused here.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AuthConfigYaml {
    /// OIDC issuer URL.
    /// For Cognito: https://cognito-idp.{region}.amazonaws.com/{userPoolId}
    pub issuer: String,
    /// OIDC JWKS URI (server-side validation — not used by the website directly).
    pub jwks_uri: String,
    /// Expected audience / client ID for token validation.
    pub audience: Option<String>,
    /// Default scopes. Falls back to `openid email profile` if absent.
    pub scopes: Option<Vec<String>>,
    /// OAuth2 client ID. Required for the PKCE login flow.
    pub client_id: Option<String>,
    /// Client secret for documentor.
    pub client_secret: Option<String>,
}

fn load_auth_config() -> Result<AuthConfigYaml, String> {
    let contents = std::fs::read_to_string(MICROKIT_YML)
        .map_err(|e| format!("Could not read microkit.yml: {e}"))?;
    let app: AppConfig = serde_yaml_ng::from_str(&contents)
        .map_err(|e| format!("Could not parse microkit.yml: {e}"))?;
    app.auth.ok_or_else(|| {
        "No 'auth' section in microkit.yml — add issuer, client_id, etc.".to_string()
    })
}

// ---------------------------------------------------------------------------
// OIDC / PKCE helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OidcDiscovery {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct OidcTokenResponse {
    access_token: String,
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    rand::rng().fill_bytes(&mut buf);
    buf
}

/// Returns `(verifier, challenge)` for PKCE S256.
fn pkce_pair() -> (String, String) {
    let verifier = URL_SAFE_NO_PAD.encode(random_bytes(64));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()).as_slice());
    (verifier, challenge)
}

fn random_state() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes(16))
}

async fn fetch_discovery(issuer: &str) -> Result<OidcDiscovery, String> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Discovery request failed: {e}"))?
        .json::<OidcDiscovery>()
        .await
        .map_err(|e| format!("Failed to parse discovery document: {e}"))
}

/// Spins up a one-shot local HTTP server and waits for the OIDC redirect.
/// Returns `(code, state)`.
async fn wait_for_callback(port: u16) -> Result<(String, String), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| format!("Could not bind callback listener on port {port}: {e}"))?;

    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(|e| format!("Callback accept failed: {e}"))?;

    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Callback read failed: {e}"))?;

    // First line: "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("");
    let query = path.split_once('?').map(|x| x.1).unwrap_or("");

    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        match kv.next() {
            Some("code") => code = kv.next().map(str::to_string),
            Some("state") => state = kv.next().map(str::to_string),
            _ => {}
        }
    }

    let body = "<html><body style='font-family:sans-serif;padding:40px'>\
        <h2>Login successful</h2><p>You can close this tab.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;

    Ok((
        code.ok_or("No 'code' in callback URL")?,
        state.ok_or("No 'state' in callback URL")?,
    ))
}

async fn exchange_code(
    token_endpoint: &str,
    client_id: &str,
    code: &str,
    code_verifier: &str,
) -> Result<String, String> {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("grant_type", "authorization_code")
        .append_pair("client_id", client_id)
        .append_pair("code", code)
        .append_pair("redirect_uri", CALLBACK_URI)
        .append_pair("code_verifier", code_verifier)
        .finish();
    reqwest::Client::new()
        .post(token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token exchange failed: {e}"))?
        .json::<OidcTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))
        .map(|r| r.access_token)
}

/// Full Authorization Code + PKCE login flow.
async fn oidc_login() -> Result<String, String> {
    let config = load_auth_config()?;

    let client_id = config
        .client_id
        .as_deref()
        .ok_or("'client_id' is missing from the auth section in microkit.yml")?;

    let discovery = fetch_discovery(&config.issuer).await?;

    let (verifier, challenge) = pkce_pair();
    let state = random_state();

    let scopes = config
        .scopes
        .as_ref()
        .map(|v| v.join(" "))
        .unwrap_or_else(|| "openid email profile".to_string());

    let mut auth_url = url::Url::parse(&discovery.authorization_endpoint)
        .map_err(|e| format!("Invalid authorization_endpoint: {e}"))?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", CALLBACK_URI)
        .append_pair("scope", &scopes)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");

    webbrowser::open(auth_url.as_str()).map_err(|e| format!("Could not open browser: {e}"))?;

    let (code, returned_state) = wait_for_callback(CALLBACK_PORT).await?;

    if returned_state != state {
        return Err("State mismatch — possible CSRF attack, aborting.".to_string());
    }

    exchange_code(&discovery.token_endpoint, client_id, &code, &verifier).await
}

// ---------------------------------------------------------------------------
// Auth context
// ---------------------------------------------------------------------------

/// Shared across the entire component tree via context.
type AuthToken = Signal<Option<String>>;

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Users {},
}

fn main() {
    dioxus::launch(App);
}

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

#[component]
fn App() -> Element {
    let token: AuthToken = use_signal(|| None::<String>);
    use_context_provider(|| token);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div {
            class: "bg-[#0f1116] text-white min-h-screen font-sans",
            if token().is_some() {
                Router::<Route> {}
            } else {
                Login {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Login page
// ---------------------------------------------------------------------------

#[component]
fn Login() -> Element {
    let mut token = use_context::<AuthToken>();
    let mut error = use_signal(|| Option::<String>::None);
    let mut logging_in = use_signal(|| false);

    rsx! {
        div {
            class: "flex flex-col items-center justify-center min-h-screen gap-4 text-center px-4",
            h1 { class: "text-4xl font-bold mb-1", "MicroKit" }
            p { class: "text-gray-500 text-sm", "Sign in to continue." }
            button {
                class: "bg-[#91a4d2] text-[#0f1116] font-semibold px-7 py-2.5 rounded transition-colors hover:bg-[#b0c0e8] disabled:opacity-60 disabled:cursor-not-allowed",
                disabled: logging_in(),
                onclick: move |_| async move {
                    *logging_in.write() = true;
                    *error.write() = None;
                    match oidc_login().await {
                        Ok(t) => *token.write() = Some(t),
                        Err(e) => {
                            *error.write() = Some(e);
                            *logging_in.write() = false;
                        }
                    }
                },
                if logging_in() { "Signing in..." } else { "Sign in" }
            }
            if let Some(e) = error() {
                p { class: "text-red-400 text-sm", "{e}" }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[component]
fn Navbar() -> Element {
    let mut token = use_context::<AuthToken>();
    rsx! {
        div {
            class: "flex flex-row items-center px-5 py-3 border-b border-[#2a2d36] mb-6",
            Link {
                class: "text-white no-underline mr-5 hover:text-[#91a4d2] transition-colors",
                to: Route::Users {},
                "Users"
            }
            div { class: "flex-1" }
            button {
                class: "border border-[#2a2d36] rounded text-[#9aa5c4] text-sm px-3 py-1 bg-transparent cursor-pointer transition-colors hover:border-[#91a4d2] hover:text-white",
                onclick: move |_| *token.write() = None,
                "Sign out"
            }
        }
        Outlet::<Route> {}
    }
}

// ---------------------------------------------------------------------------
// Users page
// ---------------------------------------------------------------------------

#[component]
fn Users() -> Element {
    let token = use_context::<AuthToken>();
    let mut name = use_signal(String::new);
    let mut status = use_signal(|| Option::<String>::None);

    let mut users = use_resource(move || async move {
        // Reading token() here means the resource re-runs if auth state changes.
        let bearer = token().unwrap_or_default();
        match reqwest::Client::new()
            .get(format!("{API_BASE}/api/v1/users"))
            .bearer_auth(bearer)
            .send()
            .await
        {
            Ok(r) => match r.json::<Vec<UserResponse>>().await {
                Ok(list) => Ok(list),
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    });

    rsx! {
        div {
            class: "max-w-4xl mx-auto px-5 py-4",

            h1 { class: "text-3xl font-semibold mb-4", "Users" }

            div {
                class: "mb-2",
                {
                    let users_read = users.read();
                    match users_read.as_ref() {
                        None => rsx! { p { class: "text-gray-500 text-sm", "Loading..." } },
                        Some(Err(e)) => rsx! { p { class: "text-red-400 text-sm", "Failed to load users: {e}" } },
                        Some(Ok(list)) => {
                            if list.is_empty() {
                                rsx! { p { class: "text-gray-500 text-sm", "No users yet." } }
                            } else {
                                rsx! {
                                    table {
                                        class: "w-full border-collapse text-sm",
                                        thead {
                                            tr {
                                                th { class: "text-left px-3 py-2 border-b border-[#2a2d36] text-[#91a4d2] font-semibold", "Name" }
                                                th { class: "text-left px-3 py-2 border-b border-[#2a2d36] text-[#91a4d2] font-semibold", "System" }
                                                th { class: "text-left px-3 py-2 border-b border-[#2a2d36] text-[#91a4d2] font-semibold", "Key" }
                                            }
                                        }
                                        tbody {
                                            for user in list.iter() {
                                                tr {
                                                    class: "group",
                                                    td { class: "px-3 py-2 border-b border-[#1e2028] group-hover:bg-[#1a1d26]", "{user.name}" }
                                                    td { class: "px-3 py-2 border-b border-[#1e2028] group-hover:bg-[#1a1d26]", "{user.creation_system}" }
                                                    td { class: "px-3 py-2 border-b border-[#1e2028] font-mono text-[#9aa5c4] group-hover:bg-[#1a1d26]", "{user.creation_key}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div {
                class: "mt-2",
                h2 { class: "text-xl font-medium mt-8 mb-2", "Create User" }
                div {
                    class: "flex gap-2.5 items-center flex-wrap mt-2.5",
                    input {
                        class: "bg-[#1a1d26] border border-[#2a2d36] rounded text-white text-sm px-2.5 py-2 outline-none flex-1 min-w-40 focus:border-[#91a4d2]",
                        r#type: "text",
                        placeholder: "Name",
                        value: "{name}",
                        oninput: move |e| *name.write() = e.value(),
                    }
                    button {
                        class: "bg-[#91a4d2] text-[#0f1116] font-semibold text-sm px-4 py-2 rounded whitespace-nowrap cursor-pointer transition-colors hover:bg-[#b0c0e8]",
                        onclick: move |_| async move {
                            *status.write() = None;
                            let bearer = token().unwrap_or_default();
                            match reqwest::Client::new()
                                .post(format!("{API_BASE}/api/v1/users"))
                                .bearer_auth(bearer)
                                .json(&UserRequest { name: name() })
                                .send()
                                .await
                            {
                                Ok(r) if r.status().is_success() => {
                                    *status.write() = Some("User created.".to_string());
                                    name.write().clear();
                                    users.restart();
                                }
                                Ok(r) => {
                                    *status.write() = Some(format!("HTTP {}", r.status()));
                                }
                                Err(e) => {
                                    *status.write() = Some(format!("Request failed: {e}"));
                                }
                            }
                        },
                        "Create"
                    }
                }
                if let Some(s) = status() {
                    p { class: "text-green-300 text-sm mt-2", "{s}" }
                }
            }
        }
    }
}
