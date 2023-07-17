use std::sync::Arc;

use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use thiserror::Error;
use tracing::info;

use crate::Context as BotContext;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
}

pub struct Context {
    pub pool: SqlitePool,
}

pub async fn init(ctx: &Arc<BotContext>) -> Result<(), DBError> {
    info!("Connecting to db: {}", ctx.config.db_url);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&ctx.config.db_url)
        .await?;

    info!("Running migrations");
    sqlx::migrate!().run(&pool).await?;

    let mut lock = ctx.modules.write().await;
    let db_ctx = &mut lock.as_mut().ok_or(DBError::InitializationError)?.database;

    *db_ctx = Some(Context { pool });
    info!("Database ready");

    Ok(())
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error(transparent)]
    Sqlx {
        source: Box<dyn std::error::Error + Sync + Send>,
    },
    #[error("`init` called before `init_modules`")]
    InitializationError,
}

impl From<sqlx::Error> for DBError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx {
            source: Box::new(e),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for DBError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::Sqlx {
            source: Box::new(e),
        }
    }
}
