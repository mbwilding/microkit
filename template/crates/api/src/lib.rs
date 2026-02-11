mod endpoints;

use microkit::{MicroKit, ServicePort};

#[tokio::main]
pub async fn start() -> anyhow::Result<()> {
    MicroKit::builder()
        .await?
        .with_logging()
        .with_database()
        .with_router()
        .with_dapr()
        .with_auth()
        .with_health_checks()
        .with_otel()
        .with_migrations::<migrations::Migrator>()
        .with_endpoints(endpoints::init_endpoints)
        .build()
        .await?
        .start(ServicePort::Api)
        .await
}
