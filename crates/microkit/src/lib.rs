pub mod config;
pub mod network;
pub mod router;

#[cfg(any(
    feature = "swagger",
    feature = "redoc",
    feature = "rapidoc",
    feature = "scalar"
))]
pub mod documentors;

#[cfg(feature = "health-checks")]
pub mod health;

#[cfg(feature = "otel")]
pub mod otel;

#[cfg(feature = "dapr")]
pub mod dapr;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "database")]
pub mod database;
#[cfg(feature = "database")]
use sea_orm::DatabaseConnection;
#[cfg(feature = "database")]
use sea_orm_migration::MigratorTrait;

#[cfg(feature = "tracing")]
use tracing_subscriber::{EnvFilter, fmt};

use anyhow::{Result, bail};
use config::Config;
use tower_http::cors::CorsLayer;
use utoipa_axum::router::OpenApiRouter;

pub enum ServicePort {
    Client,
    Api,
    Consumer,
    Other(u16),
}

impl ServicePort {
    pub fn get(&self) -> u16 {
        match self {
            ServicePort::Client => 7000,
            ServicePort::Api => 9000,
            ServicePort::Consumer => 10000,
            ServicePort::Other(port) => *port,
        }
    }

    pub fn get_with_offset(&self, port_base: u16) -> u16 {
        Self::get(self) + port_base
    }
}

pub struct MicroKit {
    pub config: Config,
    pub router: Option<OpenApiRouter>,
    #[cfg(feature = "database")]
    pub database: Option<DatabaseConnection>,
    #[cfg(feature = "dapr")]
    pub dapr: Option<dapr::Dapr>,
    #[cfg(feature = "auth")]
    pub auth: Option<auth::AuthConfig>,
}

pub struct MicroKitBuilder {
    config: Config,
    enable_router: bool,
    routes: Vec<OpenApiRouter>,
    #[cfg(feature = "tracing")]
    enable_logging: bool,
    #[cfg(feature = "database")]
    enable_database: bool,
    #[cfg(feature = "otel")]
    enable_otel: bool,
    #[cfg(feature = "health-checks")]
    enable_health_checks: bool,
    #[cfg(feature = "dapr")]
    enable_dapr: bool,
    #[cfg(feature = "auth")]
    enable_auth: bool,
}

impl MicroKit {
    /// Create a new builder with default configuration
    pub async fn builder() -> Result<MicroKitBuilder> {
        let config = config::get().await?;
        Ok(MicroKitBuilder::new(config))
    }

    /// Create a new builder with custom configuration
    pub fn builder_with_config(config: Config) -> MicroKitBuilder {
        MicroKitBuilder::new(config)
    }

    pub fn add_route(&mut self, route: OpenApiRouter) {
        match &mut self.router {
            Some(router) => self.router = Some(router.clone().merge(route)),
            None => self.router = Some(route),
        }
    }

    /// Run database migrations
    #[cfg(feature = "database")]
    pub async fn run_migrations<M: MigratorTrait>(&self) -> Result<()> {
        if let Some(database) = &self.database {
            M::up(database, None).await?;
        }
        Ok(())
    }

    pub async fn start(mut self, port_base: ServicePort) -> Result<()> {
        if let Some(router) = &mut self.router {
            #[allow(unused_mut)]
            let (mut router, api) = router.clone().split_for_parts();

            #[cfg(feature = "auth")]
            if let Some(auth) = &self.auth {
                router = router.layer(axum::middleware::from_fn_with_state(
                    auth.clone(),
                    auth::inject_auth_config,
                ));
            }

            #[allow(unused_variables)]
            let (address, listener) =
                network::network(&self.config.host, port_base, self.config.port_offset).await?;

            #[cfg(feature = "auth")]
            let router =
                documentors::documentors(router, &api, &address, self.config.auth.as_ref());

            #[cfg(all(
                any(
                    feature = "swagger",
                    feature = "redoc",
                    feature = "rapidoc",
                    feature = "scalar"
                ),
                not(feature = "auth")
            ))]
            let router = documentors::documentors(router, &api, &address);

            let router = router.layer(CorsLayer::very_permissive());

            axum::serve(listener, router.into_make_service()).await?;
        } else {
            bail!("No router");
        }

        Ok(())
    }
}

impl MicroKitBuilder {
    fn new(config: Config) -> Self {
        Self {
            config,
            enable_router: false,
            routes: Vec::new(),
            #[cfg(feature = "tracing")]
            enable_logging: false,
            #[cfg(feature = "database")]
            enable_database: false,
            #[cfg(feature = "otel")]
            enable_otel: false,
            #[cfg(feature = "health-checks")]
            enable_health_checks: false,
            #[cfg(feature = "dapr")]
            enable_dapr: false,
            #[cfg(feature = "auth")]
            enable_auth: false,
        }
    }

    /// Enable logging with the configured log level
    #[cfg(feature = "tracing")]
    pub fn with_logging(mut self) -> Self {
        self.enable_logging = true;
        self
    }

    /// Enable database connection
    #[cfg(feature = "database")]
    pub fn with_database(mut self) -> Self {
        self.enable_database = true;
        self
    }

    /// Enable router (required for serving HTTP)
    pub fn with_router(mut self) -> Self {
        self.enable_router = true;
        self
    }

    /// Add a route to the service
    pub fn add_route(mut self, route: OpenApiRouter) -> Self {
        self.enable_router = true;
        self.routes.push(route);
        self
    }

    /// Enable OpenTelemetry integration
    #[cfg(feature = "otel")]
    pub fn with_otel(mut self) -> Self {
        self.enable_otel = true;
        self
    }

    /// Enable health check endpoints
    #[cfg(feature = "health-checks")]
    pub fn with_health_checks(mut self) -> Self {
        self.enable_health_checks = true;
        self
    }

    /// Enable Dapr integration
    #[cfg(feature = "dapr")]
    pub fn with_dapr(mut self) -> Self {
        self.enable_dapr = true;
        self
    }

    /// Enable authentication
    #[cfg(feature = "auth")]
    pub fn with_auth(mut self) -> Self {
        self.enable_auth = true;
        self
    }

    /// Build the MicroKit instance with all configured features
    pub async fn build(self) -> Result<MicroKit> {
        #[cfg(feature = "tracing")]
        if self.enable_logging {
            let filter = if let Some(log_level) = &self.config.log_level {
                EnvFilter::new(log_level)
            } else {
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
            };

            let subscriber = fmt().with_env_filter(filter).finish();

            if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
                log::warn!("This will show when running in all mode ({})", e);
            } else {
                log::info!("logging initialized");
            }
        }

        // Initialize database if enabled
        #[cfg(feature = "database")]
        let database = if self.enable_database {
            Some(
                database::setup_database(
                    &self.config.database_url,
                    &self.config.database_name,
                    &self.config.database_drop,
                )
                .await?,
            )
        } else {
            None
        };

        // Initialize router if enabled
        let mut router = if self.enable_router {
            #[cfg(feature = "auth")]
            {
                // If auth config is available, create router with auth
                if let Some(auth_yaml) = &self.config.auth {
                    Some(router::generate_router_with_auth(
                        &self.config.service_name,
                        &self.config.service_desc,
                        Some(auth_yaml.issuer.clone()),
                    ))
                } else {
                    Some(router::generate_router(
                        &self.config.service_name,
                        &self.config.service_desc,
                    ))
                }
            }

            #[cfg(not(feature = "auth"))]
            Some(router::generate_router(
                &self.config.service_name,
                &self.config.service_desc,
            ))
        } else {
            None
        };

        // Add routes
        if !self.routes.is_empty() {
            for route in self.routes {
                match &mut router {
                    Some(r) => router = Some(r.clone().merge(route)),
                    None => router = Some(route),
                }
            }
        }

        // Initialize OpenTelemetry if enabled
        #[cfg(feature = "otel")]
        if self.enable_otel
            && let Some(ref mut r) = router
        {
            let router_otel = otel::init(
                r.clone().split_for_parts().0,
                &self.config.service_name,
                &self.config.otel,
            );
            router = Some(r.clone().merge(router_otel.into()));
        }

        // Initialize health checks if enabled
        #[cfg(feature = "health-checks")]
        if self.enable_health_checks
            && let Some(ref mut r) = router
        {
            let health_router = health::register_endpoints(axum::Router::new());
            router = Some(r.clone().merge(health_router.into()));
        }

        // Initialize Dapr if enabled
        #[cfg(feature = "dapr")]
        let dapr = if self.enable_dapr {
            Some(dapr::Dapr::new().await?)
        } else {
            None
        };

        // Initialize auth if enabled
        #[cfg(feature = "auth")]
        let auth = if self.enable_auth {
            let auth_config = self.config.create_auth_config()?;
            if let Some(auth) = auth_config {
                log::info!("Authentication initialized");
                Some(auth)
            } else {
                log::warn!("Authentication feature enabled but no auth config in microkit.yml");
                None
            }
        } else {
            None
        };

        Ok(MicroKit {
            config: self.config,
            router,
            #[cfg(feature = "database")]
            database,
            #[cfg(feature = "dapr")]
            dapr,
            #[cfg(feature = "auth")]
            auth,
        })
    }
}
