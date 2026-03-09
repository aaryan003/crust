//! CRUST Server Library
//! Exposes modules for testing and integration

pub mod auth;
pub mod database;
pub mod permissions;
pub mod routes;
pub mod storage;

use database::Database;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}
