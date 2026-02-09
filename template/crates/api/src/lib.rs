mod endpoints;

use endpoints::*;
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
        .build()
        .await?;

    service.run_migrations::<migrations::Migrator>().await?;

    service.add_route(users::api(&service.database)?);

    service.start(ServicePort::Api).await
}
