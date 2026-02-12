use microkit::prelude::*;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, CreationTracked)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub creation_system: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub creation_key: String,
    pub generated_on: chrono::DateTime<chrono::Utc>,

    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    /// Create an ActiveModel from an API request
    pub fn from_api(config: &microkit::config::Config, name: String) -> Self {
        Self {
            creation_system: Set(config.service_name.clone()),
            creation_key: Set(uuid::Uuid::new_v4().to_string()),
            generated_on: Set(chrono::Utc::now()),
            name: Set(name),
        }
    }

    /// Create an ActiveModel from a UserCreatedEvent contract
    pub fn from_event(event: contracts::UserCreatedEvent) -> Self {
        Self {
            creation_system: Set(event.creation_system),
            creation_key: Set(event.creation_key),
            generated_on: Set(event.generated_on),
            name: Set(event.name),
        }
    }
}
