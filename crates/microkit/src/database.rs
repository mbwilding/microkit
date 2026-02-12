use anyhow::{Result, bail};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

pub async fn setup_database(
    url: &Option<String>,
    name: &Option<String>,
    drop: &Option<bool>,
) -> Result<DatabaseConnection> {
    let url = match url {
        Some(url) => url,
        None => bail!("database_url not set"),
    };

    let name = match name {
        Some(name) => name,
        None => bail!("database_name not set"),
    };

    tracing::info!("database: connecting to root database");
    let db = Database::connect(url).await?;

    if let Some(true) = drop {
        db.execute_unprepared(&format!("DROP DATABASE IF EXISTS \"{}\";", name))
            .await?;

        db.execute_unprepared(&format!("CREATE DATABASE \"{}\";", name))
            .await?;
    } else {
        let exists_sql = format!(
            "SELECT 1 FROM pg_database WHERE datname = '{}';",
            name.replace("'", "''")
        );

        let stmt =
            Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, &exists_sql, vec![]);
        let exists = db.query_one_raw(stmt).await?.is_some();

        if !exists {
            db.execute_unprepared(&format!("CREATE DATABASE \"{}\";", name))
                .await?;
        }
    }

    tracing::info!("connecting to database '{}'", &name);
    let url = format!("{}/{}", &url, &name);
    Ok(Database::connect(&url).await?)
}
