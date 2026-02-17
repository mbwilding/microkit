/// Trait for entities that track creation metadata with composite keys
pub trait CreationTracking {
    /// Get the creation system (service name)
    fn creation_system(&self) -> &str;

    /// Get the creation key (UUID or external system identifier)
    fn creation_key(&self) -> &str;
}

/// Helper trait for creating ActiveModels from API requests
///
/// This is automatically used by entities with creation tracking.
/// It auto-generates creation_system from config and creation_key as UUID.
pub trait FromApiRequest<T> {
    type Error;

    /// Create an ActiveModel from an API request payload
    ///
    /// Automatically sets:
    /// - creation_system from config.service_name
    /// - creation_key as a new UUID
    fn from_api(config: &crate::config::Config, payload: T) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Helper trait for creating ActiveModels from event contracts
///
/// This is automatically used by entities with creation tracking.
/// It uses the explicit creation tracking from the event.
pub trait FromEventContract<T> {
    type Error;

    /// Create an ActiveModel from an event contract
    ///
    /// Uses the creation_system and creation_key from the event.
    fn from_event(contract: T) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Macro to add creation tracking fields to SeaORM entities
#[macro_export]
macro_rules! creation_tracking_fields {
    () => {
        /// System that created this record (e.g. service name)
        #[sea_orm(primary_key, auto_increment = false)]
        pub creation_system: String,

        /// Unique identifier - UUID for API, message ID for events
        #[sea_orm(primary_key, auto_increment = false)]
        pub creation_key: String,
    };
}

/// Macro to add creation tracking columns to migrations
#[macro_export]
macro_rules! creation_tracking_columns {
    () => {
        |table: &mut sea_orm_migration::prelude::TableCreateStatement| {
            table
                .col(
                    sea_orm_migration::prelude::ColumnDef::new(
                        sea_orm_migration::prelude::Alias::new("creation_system"),
                    )
                    .string()
                    .not_null(),
                )
                .col(
                    sea_orm_migration::prelude::ColumnDef::new(
                        sea_orm_migration::prelude::Alias::new("creation_key"),
                    )
                    .string()
                    .not_null(),
                )
                .primary_key(
                    sea_orm_migration::prelude::Index::create()
                        .col(sea_orm_migration::prelude::Alias::new("creation_system"))
                        .col(sea_orm_migration::prelude::Alias::new("creation_key")),
                )
        }
    };
}
