//! Database module - PostgreSQL connection management

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    /// Create a new database connection pool from DATABASE_URL
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await?;

        Ok(Database { pool })
    }

    /// Health check - verify database is accessible
    pub async fn health_check(&self) -> Result<DatabaseHealth, String> {
        let start = std::time::Instant::now();

        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let elapsed = start.elapsed();

        Ok(DatabaseHealth {
            connected: true,
            response_time_ms: elapsed.as_millis() as u64,
            pool_size: self.pool.num_idle() as u32,
        })
    }
}

/// Database health status
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub response_time_ms: u64,
    pub pool_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_health_serializes() {
        let health = DatabaseHealth {
            connected: true,
            response_time_ms: 42,
            pool_size: 3,
        };
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"connected\":true"));
        assert!(json.contains("\"response_time_ms\":42"));
    }
}
