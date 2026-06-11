use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use mesh_core::graph_engine::GraphEngine;

/// Estado compartido de la aplicación.
/// Contiene el pool de conexiones a PostgreSQL
/// y el grafo en memoria protegido por un RwLock.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub motor: Arc<RwLock<GraphEngine>>,
}

impl AppState {
    /// Inicializa el AppState.
    /// Carga la topología de servicios activos y dependencias
    /// desde PostgreSQL hacia el grafo en memoria.
    pub async fn new(db: PgPool) -> Result<Self, sqlx::Error> {
        let motor = Arc::new(RwLock::new(GraphEngine::new()));

        // Cargar solo servicios activos desde la BD
        let servicios = crate::db::cargar_nombres_servicios_activos(&db).await?;

        {
            let mut m = motor.write().await;
            for nombre in &servicios {
                m.agregar_servicio(nombre);
            }
        }

        // Cargar dependencias existentes desde la BD
        let dependencias = crate::db::cargar_pares_dependencias(&db).await?;

        {
            let mut m = motor.write().await;
            for (origen, destino) in &dependencias {
                m.agregar_dependencia(origen, destino);
            }
        }

        tracing::info!(
            "Topología cargada: {} servicios, {} dependencias.",
            servicios.len(),
            dependencias.len()
        );

        Ok(Self { db, motor })
    }
}