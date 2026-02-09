use axum::Router;
use axum::response::Html;
use axum::routing::get;

pub fn register_endpoints(router: Router) -> Router {
    router.merge(
        Router::new()
            .route("/status/ready", get(Html("ready")))
            .route("/status/live", get(Html("live"))),
    )
}
