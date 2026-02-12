use axum::{Json, extract::State, http::StatusCode};
use contracts::UserCreatedEvent;
use entities::users::ActiveModel;
use microkit::prelude::*;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

const GROUP: &str = "Users (CONSUMER)";
const PATH: &str = "/v1/event/users";

/// User response
#[event_contract]
#[derive(Debug, Serialize, ToSchema)]
pub struct DaprUserResponse {
    pub name: String,
}

/// Create user
#[tracing::instrument(skip(db))]
#[utoipa::path(
    post,
    path = PATH,
    tag = GROUP,
    request_body = UserCreatedEvent,
    responses(
        (status = 200, description = "User created from event", body = DaprUserResponse),
        (status = 400, description = "Bad request - missing required fields"),
        (status = 409, description = "Conflict - user with this creation_system/creation_key already exists")
    )
)]
pub async fn create_user_from_event(
    State(db): State<DatabaseConnection>,
    Json(event): Json<UserCreatedEvent>,
) -> Result<Json<DaprUserResponse>, StatusCode> {
    if event.creation_system.is_empty() || event.creation_key.is_empty() {
        tracing::error!("Missing required creation tracking fields");
        return Err(StatusCode::BAD_REQUEST);
    }

    tracing::info!(
        creation_system = %event.creation_system,
        creation_key = %event.creation_key,
        generated_on = %event.generated_on,
        name = %event.name,
        "Creating user from Dapr event"
    );

    let active_model = ActiveModel::from_event(event);

    let inserted = active_model.insert(&db).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to insert user from event");
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    tracing::info!(
        creation_system = %inserted.creation_system,
        creation_key = %inserted.creation_key,
        "User created successfully from event"
    );

    Ok(Json(DaprUserResponse {
        creation_system: inserted.creation_system,
        creation_key: inserted.creation_key,
        generated_on: inserted.generated_on,
        name: inserted.name,
    }))
}
