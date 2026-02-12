use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::CreationSystem).string().not_null())
                    .col(ColumnDef::new(Users::CreationKey).string().not_null())
                    .col(
                        ColumnDef::new(Users::GeneratedOn)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .primary_key(
                        Index::create()
                            .col(Users::CreationSystem)
                            .col(Users::CreationKey),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    CreationSystem,
    CreationKey,
    GeneratedOn,
    Name,
}
