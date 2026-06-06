use sqlx::{PgPool, Row};
use std::sync::{Arc, Mutex};
use mesh_core::dfs::Grafo;

/// Estado compartido de la aplicación.
/// Contiene el pool de conexiones a PostgreSQL
/// y el grafo en memoria protegido por un Mutex.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub grafo: Arc<Mutex<Grafo>>,
}

impl AppState {
    /// Inicializa el AppState.
    /// Carga la topología de servicios activos y dependencias
    /// desde PostgreSQL hacia el grafo en memoria.
    pub async fn new(db: PgPool) -> Result<Self, sqlx::Error> {
        let grafo = Arc::new(Mutex::new(Grafo::nuevo()));

        // Cargar solo servicios activos desde la BD
        let servicios = sqlx::query("SELECT nombre FROM servicios WHERE activo = TRUE")
            .fetch_all(&db)
            .await?;

        {
            let mut g = grafo.lock().unwrap();
            for s in &servicios {
                let nombre: String = s.try_get("nombre")?;
                g.agregar_servicio(&nombre);
            }
        }

        // Cargar dependencias existentes desde la BD
        let dependencias = sqlx::query("SELECT origen, destino FROM dependencias")
            .fetch_all(&db)
            .await?;

        {
            let mut g = grafo.lock().unwrap();
            for d in &dependencias {
                let origen: String = d.try_get("origen")?;
                let destino: String = d.try_get("destino")?;
                g.agregar_dependencia(&origen, &destino);
            }
        }

        tracing::info!(
            "Topología cargada: {} servicios, {} dependencias.",
            servicios.len(),
            dependencias.len()
        );

        Ok(Self { db, grafo })
    }
}