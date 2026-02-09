use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

#[cfg(feature = "otel")]
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

#[cfg(feature = "auth")]
use utoipa::openapi::security::{OpenIdConnect, SecurityScheme};

#[cfg(feature = "auth")]
pub fn generate_router_with_auth(
    title: &str,
    description: &str,
    issuer: Option<String>,
) -> OpenApiRouter {
    let mut router = generate_router_base(title, description);
    let mut components = utoipa::openapi::ComponentsBuilder::new();

    // components = components.security_scheme(
    //     "bearer",
    //     SecurityScheme::Http(
    //         HttpBuilder::new()
    //             .scheme(HttpAuthScheme::Bearer)
    //             .bearer_format("JWT")
    //             .description(Some("JWT Bearer token from Cognito/OIDC provider"))
    //             .build(),
    //     ),
    // );

    if let Some(issuer_url) = &issuer {
        // let token_url = format!("{}/oauth2/token", issuer_url);
        // let auth_url = format!("{}/oauth2/authorize", issuer_url);
        //
        // let scopes = Scopes::from_iter([
        //     ("openid", "OpenID Connect scope"),
        //     ("email", "Email address"),
        //     ("profile", "User profile information"),
        // ]);
        //
        // let client_creds_flow =
        //     Flow::ClientCredentials(ClientCredentials::new(token_url.clone(), scopes.clone()));
        //
        // let auth_code_flow =
        //     Flow::AuthorizationCode(AuthorizationCode::new(auth_url, token_url, scopes));
        //
        // components = components.security_scheme(
        //     "oauth2",
        //     SecurityScheme::OAuth2(OAuth2::new([client_creds_flow, auth_code_flow])),
        // );

        components = components.security_scheme(
            "oidc",
            SecurityScheme::OpenIdConnect(OpenIdConnect::new(format!(
                "{}/.well-known/openid-configuration",
                issuer_url
            ))),
        );
    }

    let openapi = router.get_openapi_mut();
    openapi.components = Some(components.build());

    apply_otel_if_enabled(router)
}

pub fn generate_router(title: &str, description: &str) -> OpenApiRouter {
    let router = generate_router_base(title, description);
    apply_otel_if_enabled(router)
}

fn generate_router_base(title: &str, description: &str) -> OpenApiRouter {
    #[derive(OpenApi)]
    struct ApiDoc;

    let mut openapi = ApiDoc::openapi();
    openapi.info.title = title.to_string();
    openapi.info.description = Some(description.to_string());

    OpenApiRouter::with_openapi(openapi)
}

fn apply_otel_if_enabled(router: OpenApiRouter) -> OpenApiRouter {
    #[cfg(feature = "otel")]
    let router = router
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default());

    router
}
