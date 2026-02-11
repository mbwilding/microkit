mod endpoints;

use microkit::{MicroKit, ServicePort};

#[tokio::main]
pub async fn start() -> anyhow::Result<()> {
    let mut service = MicroKit::builder()
        .await?
        .with_logging()
        .with_database()
        .with_router()
        .with_dapr()
        .with_auth()
        .with_health_checks()
        .with_otel()
        .build()
        .await?;

    service.run_migrations::<migrations::Migrator>().await?;

    endpoints::init_endpoints(&mut service)?;

    service.start(ServicePort::Api).await
}
