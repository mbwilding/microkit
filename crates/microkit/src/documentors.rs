use axum::Router;
use std::net::SocketAddr;
use utoipa::openapi::OpenApi;

#[cfg(feature = "auth")]
use crate::config::AuthConfigYaml;

#[cfg(feature = "auth")]
pub fn documentors(
    router: Router,
    api: &OpenApi,
    local_addr: &SocketAddr,
    auth_config: Option<&AuthConfigYaml>,
) -> Router {
    let mut router = router;
    let mut documentors: Vec<&str> = Vec::with_capacity(4);

    // Documentation endpoints
    {
        #[allow(unused_variables)]
        let openapi_json = "/api-docs/openapi.json";

        #[cfg(feature = "swagger")]
        {
            use utoipa_swagger_ui::{SwaggerUi, oauth};
            let endpoint = "/swagger";

            let mut swagger_ui = SwaggerUi::new(endpoint).url(openapi_json, api.clone());

            // Configure OAuth2 if auth is available
            if let Some(auth) = auth_config {
                let mut oauth_config =
                    oauth::Config::new().use_pkce_with_authorization_code_grant(false);

                if let Some(client_id) = &auth.client_id {
                    oauth_config = oauth_config.client_id(client_id);
                }

                if let Some(client_secret) = &auth.client_secret {
                    oauth_config = oauth_config.client_secret(client_secret);
                }

                if let Some(scopes) = &auth.scopes {
                    oauth_config = oauth_config.scopes(scopes.clone());
                }

                swagger_ui = swagger_ui.oauth(oauth_config);
            }

            router = router.merge(swagger_ui);
            documentors.push(endpoint);
        }

        #[cfg(feature = "redoc")]
        {
            use utoipa_redoc::{Redoc, Servable};
            let endpoint = "/redoc";
            router = router.merge(Redoc::with_url(endpoint, api.clone()));
            documentors.push(endpoint);
        }

        #[cfg(feature = "rapidoc")]
        {
            use utoipa_rapidoc::RapiDoc;
            let endpoint = "/rapidoc";
            router = router.merge(RapiDoc::new(openapi_json).path(endpoint));
            documentors.push(endpoint);
        }

        #[cfg(feature = "scalar")]
        {
            use utoipa_scalar::{Scalar, Servable as ScalarServable};
            let endpoint = "/scalar";
            router = router.merge(Scalar::with_url(endpoint, api.clone()));
            documentors.push(endpoint);
        }
    }

    // Documentation viewers
    for documentor in documentors {
        let name = &documentor[1..];
        log::info!("{}: http://{}/{}", name, local_addr, name);
    }

    router
}

#[cfg(not(feature = "auth"))]
pub fn documentors(router: Router, api: &OpenApi, local_addr: &SocketAddr) -> Router {
    let mut router = router;
    let mut documentors: Vec<&str> = Vec::with_capacity(4);

    // Documentation endpoints
    {
        #[allow(unused_variables)]
        let openapi_json = "/api-docs/openapi.json";

        #[cfg(feature = "swagger")]
        {
            use utoipa_swagger_ui::SwaggerUi;
            let endpoint = "/swagger";
            router = router.merge(SwaggerUi::new(endpoint).url(openapi_json, api.clone()));
            documentors.push(endpoint);
        }

        #[cfg(feature = "redoc")]
        {
            use utoipa_redoc::{Redoc, Servable};
            let endpoint = "/redoc";
            router = router.merge(Redoc::with_url(endpoint, api.clone()));
            documentors.push(endpoint);
        }

        #[cfg(feature = "rapidoc")]
        {
            use utoipa_rapidoc::RapiDoc;
            let endpoint = "/rapidoc";
            router = router.merge(RapiDoc::new(openapi_json).path(endpoint));
            documentors.push(endpoint);
        }

        #[cfg(feature = "scalar")]
        {
            use utoipa_scalar::{Scalar, Servable as ScalarServable};
            let endpoint = "/scalar";
            router = router.merge(Scalar::with_url(endpoint, api.clone()));
            documentors.push(endpoint);
        }
    }

    // Documentation viewers
    for documentor in documentors {
        let name = &documentor[1..];
        log::info!("{}: http://{}/{}", name, local_addr, name);
    }

    router
}
