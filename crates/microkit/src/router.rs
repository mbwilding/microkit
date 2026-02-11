use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

#[cfg(feature = "auth")]
use utoipa::openapi::security::{OpenIdConnect, SecurityScheme};

#[cfg(feature = "auth")]
pub fn generate_router_with_auth(
    title: &str,
    description: &Option<String>,
    issuer: Option<String>,
) -> OpenApiRouter {
    let mut router = generate_router(title, description);
    let mut components = utoipa::openapi::ComponentsBuilder::new();

    if let Some(issuer_url) = &issuer {
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

    router
}

pub fn generate_router(title: &str, description: &Option<String>) -> OpenApiRouter {
    #[derive(OpenApi)]
    struct ApiDoc;

    let mut openapi = ApiDoc::openapi();
    openapi.info.title = title.to_string();
    openapi.info.description = description.clone();

    OpenApiRouter::with_openapi(openapi)
}
