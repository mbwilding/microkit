use microkit::event_contract;
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Event representing a user creation
#[event_contract]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserCreatedEvent {
    /// User's name
    pub name: String,
}
