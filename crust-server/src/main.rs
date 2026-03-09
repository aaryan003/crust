//! CRUST HTTP server

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Router,
};
use crust_server::{auth::handlers, database::Database, routes, AppState};
use serde_json::json;
use std::sync::Arc;

/// Health check response
async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(db_health) => {
            let response = json!({
                "status": "ok",
                "service": "crust-server",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "database": {
                    "connected": db_health.connected,
                    "response_time_ms": db_health.response_time_ms,
                    "pool_size": db_health.pool_size,
                }
            });
            (StatusCode::OK, axum::Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "service": "crust-server",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "database": { "connected": false }
            });
            (StatusCode::SERVICE_UNAVAILABLE, axum::Json(response))
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crust".to_string());

    // Initialize database connection pool
    let db = match Database::new(&database_url).await {
        Ok(db) => {
            tracing::info!("Database connected successfully");
            db
        }
        Err(e) => {
            tracing::warn!("Database connection failed (continuing without DB): {}", e);
            eprintln!("Warning: Database not available. Health check will report degraded status.");
            std::process::exit(1);
        }
    };

    // Run database migrations automatically on startup
    tracing::info!("Running database migrations...");
    match sqlx::migrate!("./migrations").run(&db.pool).await {
        Ok(()) => tracing::info!("Migrations applied successfully"),
        Err(e) => tracing::warn!("Migration warning (may already be applied): {}", e),
    }

    let state = Arc::new(AppState { db });

    // Build router with auth and repository routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/auth/register", post(handlers::register))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/logout", post(handlers::logout))
        .route("/api/v1/auth/me", get(handlers::me))
        .route("/api/v1/repos", post(routes::create_repository))
        .route("/api/v1/repos/:owner/:repo", get(routes::get_repository))
        .route(
            "/api/v1/repos/:owner/:repo",
            patch(routes::update_repository),
        )
        .route(
            "/api/v1/repos/:owner/:repo",
            delete(routes::delete_repository),
        )
        .route(
            "/api/v1/repos/:owner/:repo/refs",
            get(routes::objects::list_refs_handler),
        )
        .route(
            "/api/v1/repos/:owner/:repo/refs/preflight",
            post(routes::objects::preflight_handler),
        )
        .route(
            "/api/v1/repos/:owner/:repo/objects/upload",
            post(routes::objects::upload_handler),
        )
        .route(
            "/api/v1/repos/:owner/:repo/objects/fetch",
            post(routes::objects::fetch_handler),
        )
        .route(
            "/api/v1/repos/:owner/:repo/refs/update",
            post(routes::objects::update_refs_handler),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls",
            post(routes::prs::create_pull_request),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls",
            get(routes::prs::list_pull_requests),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls/:number",
            get(routes::prs::get_pull_request),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls/:number",
            patch(routes::prs::update_pull_request),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls/:number/reviews",
            post(routes::prs::create_review),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls/:number/comments",
            post(routes::prs::create_comment),
        )
        .route(
            "/api/v1/repos/:owner/:repo/pulls/:number/merge",
            post(routes::prs::merge_pull_request),
        )
        .route("/api/v1/orgs", post(routes::orgs::create_organization))
        .route("/api/v1/orgs/:org", get(routes::orgs::get_organization))
        .route(
            "/api/v1/orgs/:org/members",
            get(routes::orgs::list_organization_members),
        )
        .route(
            "/api/v1/orgs/:org/members/:username",
            post(routes::orgs::add_organization_member),
        )
        .route(
            "/api/v1/orgs/:org/members/:username",
            delete(routes::orgs::remove_organization_member),
        )
        .route("/api/v1/orgs/:org/teams", post(routes::teams::create_team))
        .route("/api/v1/orgs/:org/teams", get(routes::teams::list_teams))
        .route(
            "/api/v1/orgs/:org/teams/:team/repos/:owner/:repo",
            put(routes::teams::grant_team_access),
        )
        .route(
            "/api/v1/orgs/:org/teams/:team/members/:username",
            post(routes::teams::add_team_member),
        )
        .with_state(state);

    // Get port from environment
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await.expect("Server failed");
}
