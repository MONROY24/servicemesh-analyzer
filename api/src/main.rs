mod state;
mod routes;
mod handlers;
mod error;

use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Conectar a PostgreSQL
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL debe estar definida en .env");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("No se pudo conectar a PostgreSQL");

    tracing::info!("Conexión a PostgreSQL establecida.");

    let state = AppState::new(db_pool).await
        .expect("Error al inicializar AppState");

    let app = routes::crear_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("No se pudo enlazar al puerto 3000");

    tracing::info!("Servidor escuchando en http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("Error al iniciar el servidor");
}