use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct AppError {
    pub status: StatusCode,
    pub mensaje: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError { status: StatusCode::BAD_REQUEST, mensaje: msg.into() }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError { status: StatusCode::NOT_FOUND, mensaje: msg.into() }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError { status: StatusCode::CONFLICT, mensaje: msg.into() }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError { status: StatusCode::INTERNAL_SERVER_ERROR, mensaje: msg.into() }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("[{}] {}", self.status, self.mensaje);

        (
            self.status,
            Json(json!({
                "error":  self.status.canonical_reason().unwrap_or("Error"),
                "detalle": self.mensaje
            })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        // Detectar violación de restricción única (código 23505) y self-loop (23514)
        if let sqlx::Error::Database(ref db_err) = e {
            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    "23505" => return AppError::conflict("Ya existe un registro con esos datos."),
                    "23503" => return AppError::bad_request("Uno de los servicios referenciados no existe."),
                    "23514" => return AppError::bad_request("Violación de restricción CHECK (ej. self-loop o nombre vacío)."),
                    _ => {}
                }
            }
        }
        AppError::internal(e.to_string())
    }
}