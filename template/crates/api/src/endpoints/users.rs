use axum::{Json, extract::State};
use entities::users::{ActiveModel, Entity, Model};
use microkit::auth::AuthenticatedUser;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const GROUP: &str = "Users";
const PATH: &str = "/v1/users";

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UserRequest {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
}

/// Get users
#[tracing::instrument(skip(db))]
#[utoipa::path(
    get,
    path = PATH,
    tag = GROUP,
    responses(
        (status = 200, description = "List of users", body = [UserResponse])
    )
)]
pub async fn get_users(State(db): State<DatabaseConnection>) -> Json<Vec<UserResponse>> {
    let names = Entity::find().all(&db).await.unwrap();
    let responses = names
        .into_iter()
        .map(|n| UserResponse {
            id: n.id,
            name: n.name,
        })
        .collect();

    Json(responses)
}

/// Create new user
#[tracing::instrument(skip(auth_user, db))]
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
pub async fn create_user(
    auth_user: AuthenticatedUser,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<UserRequest>,
) -> Json<UserResponse> {
    // Log who is creating the user
    tracing::info!(
        user_id = %auth_user.sub,
        email = ?auth_user.email,
        groups = ?auth_user.groups,
        "User creating new user"
    );

    // if !auth_user.has_role("admin") {
    //     return Err((StatusCode::FORBIDDEN, "Requires admin role"));
    // }

    let active_model = ActiveModel {
        name: Set(payload.name),
        ..Default::default()
    };
    let inserted: Model = active_model.insert(&db).await.unwrap();

    Json(UserResponse {
        id: inserted.id,
        name: inserted.name,
    })
}
