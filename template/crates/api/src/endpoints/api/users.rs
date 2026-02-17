use axum::{Extension, Json, extract::State};
use entities::users::{ActiveModel, Entity, Model};
use microkit::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const GROUP: &str = "Users (API)";
const PATH: &str = "/api/v1/users";

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UserRequest {
    pub name: String,
}

#[api_contract]
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub name: String,
}

/// Get users
// #[tracing::instrument(skip(db))]
#[tracing::instrument()]
#[utoipa::path(
    get,
    path = PATH,
    tag = GROUP,
    responses(
        (status = 200, description = "List of users", body = [UserResponse])
    )
)]
pub async fn api_get_users(State(db): State<DatabaseConnection>) -> Json<Vec<UserResponse>> {
    let users = Entity::find().all(&db).await.unwrap();
    let responses = users
        .into_iter()
        .map(|u| UserResponse {
            creation_system: u.creation_system,
            creation_key: u.creation_key,
            name: u.name,
        })
        .collect();

    Json(responses)
}

/// Create user
// #[tracing::instrument(skip(auth_user, config, db))]
#[tracing::instrument()]
#[utoipa::path(
    post,
    path = PATH,
    tag = GROUP,
    request_body = UserRequest,
    responses(
        (status = 200, description = "User inserted", body = UserResponse),
        (status = 401, description = "Unauthorized - Invalid or missing bearer token")
    ),
    security(
        ("bearer" = []),
        ("oauth2" = ["openid", "email", "profile"]),
        ("oidc" = ["openid", "email", "profile"])
    )
)]
pub async fn api_create_user(
    auth_user: AuthenticatedUser,
    Extension(config): Extension<Config>,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<UserRequest>,
) -> Json<UserResponse> {
    tracing::info!(
        user_id = %auth_user.sub,
        email = ?auth_user.email,
        groups = ?auth_user.groups,
        "User creating new user via API"
    );

    let active_model = ActiveModel::from_api(&config, payload.name);
    let inserted: Model = active_model.insert(&db).await.unwrap();

    Json(UserResponse {
        creation_system: inserted.creation_system,
        creation_key: inserted.creation_key,
        name: inserted.name,
    })
}
