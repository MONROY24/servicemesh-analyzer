use axum::{
    routing::{get, post, delete},
    Router,
};

use crate::handlers;
use crate::state::AppState;

pub fn crear_router(state: AppState) -> Router {
    Router::new()
        // Servicios
        .route("/services",      post(handlers::registrar_servicio)
                                .get(handlers::listar_servicios))
        .route("/services/{nombre}", delete(handlers::desactivar_servicio))
        .route("/services/raiz", get(handlers::servicios_raiz))
        .route("/services/hoja", get(handlers::servicios_hoja))
        // Dependencias
        .route("/deps",          post(handlers::registrar_dependencia)
                                .get(handlers::listar_dependencias))
        .route("/deps/{origen}/{destino}", delete(handlers::eliminar_dependencia))
        // Análisis
        .route("/analyze",         get(handlers::analizar_grafo))
        .route("/analyze/history", get(handlers::historial_analisis))
        .route("/analyze/ultimo",  get(handlers::ultimo_analisis))
        .with_state(state)
}