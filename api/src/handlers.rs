use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::state::AppState;
use crate::error::AppError;
use crate::db;
use mesh_core::models::{Service, Dependency};

// ─── Helpers de serialización de fechas ──────────────────────────────────────

fn serialize_timestamp<S>(ts: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&ts.to_string())
}



// ─── DTOs de entrada ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NuevoServicio {
    pub nombre: String,
    pub descripcion: Option<String>,
}

#[derive(Deserialize)]
pub struct NuevaDependencia {
    pub origen: String,
    pub destino: String,
    pub descripcion: Option<String>,
}

// ─── DTOs de salida ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServicioDto {
    pub id: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
    #[serde(serialize_with = "serialize_timestamp")]
    pub creado_en: OffsetDateTime,
    #[serde(serialize_with = "serialize_timestamp")]
    pub actualizado_en: OffsetDateTime,
}

#[derive(Serialize)]
pub struct DependenciaDto {
    pub id: String,
    pub origen: String,
    pub destino: String,
    pub descripcion: Option<String>,
    #[serde(serialize_with = "serialize_timestamp")]
    pub creado_en: OffsetDateTime,
}

#[derive(Serialize)]
pub struct FilaVistaGrafo {
    pub dep_id: String,
    pub origen: String,
    pub desc_origen: Option<String>,
    pub destino: String,
    pub desc_destino: Option<String>,
    pub desc_dependencia: Option<String>,
    #[serde(serialize_with = "serialize_timestamp")]
    pub creado_en: OffsetDateTime,
}

#[derive(Serialize)]
pub struct ServicioResumen {
    pub nombre: String,
    pub descripcion: Option<String>,
}

#[derive(Serialize)]
pub struct AnalisisDto {
    pub id: String,
    pub tiene_ciclo: bool,
    pub snapshot_grafo: Value,
    pub ciclos_detectados: Value,
    #[serde(serialize_with = "serialize_timestamp")]
    pub ejecutado_en: OffsetDateTime,
    pub alerta: String,
}

#[derive(Serialize)]
pub struct AnalisisResumen {
    pub id: String,
    pub tiene_ciclo: bool,
    #[serde(serialize_with = "serialize_timestamp")]
    pub ejecutado_en: OffsetDateTime,
}

// ─── POST /services ───────────────────────────────────────────────────────────
/// Registra un nuevo microservicio en la BD y en el grafo en memoria.
/// Si el nombre ya existe, agrega un sufijo numérico (_1, _2, …) hasta encontrar uno disponible.
pub async fn registrar_servicio(
    State(state): State<AppState>,
    Json(payload): Json<NuevoServicio>,
) -> Result<(StatusCode, Json<ServicioDto>), AppError> {
    let servicio_modelo = Service::new(uuid::Uuid::nil(), payload.nombre.clone())
        .map_err(AppError::bad_request)?;

    let nombre_base = servicio_modelo.name;

    // Intentar insertar con el nombre original primero
    let mut nombre_final = nombre_base.clone();
    let mut db_svc = db::insertar_servicio(&state.db, &nombre_final, &payload.descripcion).await?;

    // Si ya existe, probar con sufijos _1, _2, _3, …
    if db_svc.is_none() {
        let mut sufijo = 1u32;
        loop {
            nombre_final = format!("{}_{}", nombre_base, sufijo);
            db_svc = db::insertar_servicio(&state.db, &nombre_final, &payload.descripcion).await?;
            if db_svc.is_some() {
                break;
            }
            sufijo += 1;
            if sufijo > 1000 {
                return Err(AppError::internal("No se pudo generar un nombre único después de 1000 intentos.".to_string()));
            }
        }
    }

    let db_svc = db_svc.unwrap();

    // Actualizar motor en memoria
    {
        let mut m = state.motor.write().await;
        m.agregar_servicio(&db_svc.nombre);
    }

    tracing::info!("Servicio registrado: {}", db_svc.nombre);

    Ok((
        StatusCode::CREATED,
        Json(ServicioDto {
            id: db_svc.id.to_string(),
            nombre: db_svc.nombre,
            descripcion: db_svc.descripcion,
            activo: db_svc.activo,
            creado_en: db_svc.creado_en,
            actualizado_en: db_svc.actualizado_en,
        }),
    ))
}

// ─── DELETE /services/:nombre ─────────────────────────────────────────────────
pub async fn desactivar_servicio(
    State(state): State<AppState>,
    axum::extract::Path(nombre): axum::extract::Path<String>,
) -> Result<StatusCode, AppError> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(AppError::bad_request("El nombre del servicio no puede estar vacío."));
    }

    let desactivado = db::desactivar_servicio(&state.db, nombre).await?;
    if !desactivado {
        return Err(AppError::not_found("Servicio no encontrado o ya estaba desactivado."));
    }

    // Actualizar motor en memoria
    {
        let mut m = state.motor.write().await;
        m.remover_servicio(nombre);
    }

    tracing::info!("Servicio desactivado: {}", nombre);

    Ok(StatusCode::NO_CONTENT)
}

// ─── GET /services ────────────────────────────────────────────────────────────
/// Lista todos los servicios activos.
pub async fn listar_servicios(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioDto>>, AppError> {
    let db_servicios = db::listar_servicios_activos(&state.db).await?;

    let servicios: Vec<ServicioDto> = db_servicios
        .into_iter()
        .map(|f| ServicioDto {
            id: f.id.to_string(),
            nombre: f.nombre,
            descripcion: f.descripcion,
            activo: f.activo,
            creado_en: f.creado_en,
            actualizado_en: f.actualizado_en,
        })
        .collect();

    Ok(Json(servicios))
}

// ─── GET /services/raiz ───────────────────────────────────────────────────────
/// Servicios activos que no tienen ninguna dependencia entrante (raíces del grafo).
pub async fn servicios_raiz(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioResumen>>, AppError> {
    let lista_db = db::listar_servicios_raiz(&state.db).await?;

    let lista: Vec<ServicioResumen> = lista_db
        .into_iter()
        .map(|f| ServicioResumen {
            nombre: f.nombre,
            descripcion: f.descripcion,
        })
        .collect();

    Ok(Json(lista))
}

// ─── GET /services/hoja ───────────────────────────────────────────────────────
/// Servicios activos sin dependencias salientes (hojas del grafo).
pub async fn servicios_hoja(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioResumen>>, AppError> {
    let lista_db = db::listar_servicios_hoja(&state.db).await?;

    let lista: Vec<ServicioResumen> = lista_db
        .into_iter()
        .map(|f| ServicioResumen {
            nombre: f.nombre,
            descripcion: f.descripcion,
        })
        .collect();

    Ok(Json(lista))
}

// ─── POST /deps ───────────────────────────────────────────────────────────────
/// Registra una dependencia dirigida entre dos microservicios.
/// Si la dependencia ya existe, agrega un sufijo numérico al destino (_1, _2, …)
/// para crear una variante única.
pub async fn registrar_dependencia(
    State(state): State<AppState>,
    Json(payload): Json<NuevaDependencia>,
) -> Result<(StatusCode, Json<DependenciaDto>), AppError> {
    let dependencia_modelo = Dependency::new(payload.origen.clone(), payload.destino.clone())
        .map_err(AppError::bad_request)?;

    let mut tx = state.db.begin().await.map_err(|e| AppError::internal(e.to_string()))?;

    let origen = dependencia_modelo.from_service.as_str();
    let destino_base = dependencia_modelo.to_service.clone();

    // Intentar insertar con el destino original primero
    let mut destino_final = destino_base.clone();
    let mut db_dep = db::insertar_dependencia(&mut tx, origen, &destino_final, &payload.descripcion).await?;

    // Si ya existe, probar con sufijos _1, _2, _3, …
    if db_dep.is_none() {
        let mut sufijo = 1u32;
        loop {
            destino_final = format!("{}_{}", destino_base, sufijo);
            db_dep = db::insertar_dependencia(&mut tx, origen, &destino_final, &payload.descripcion).await?;
            if db_dep.is_some() {
                break;
            }
            sufijo += 1;
            if sufijo > 1000 {
                return Err(AppError::internal("No se pudo generar una dependencia única después de 1000 intentos.".to_string()));
            }
        }
    }

    let db_dep = db_dep.unwrap();

    // Actualizar motor en memoria
    {
        let mut m = state.motor.write().await;
        m.agregar_dependencia(&db_dep.origen, &db_dep.destino);
    }

    if let Err(e) = tx.commit().await {
        // Rollback del motor si la query falla al confirmar la transacción
        let mut m = state.motor.write().await;
        m.remover_dependencia(&db_dep.origen, &db_dep.destino);
        return Err(AppError::internal(e.to_string()));
    }

    tracing::info!("Dependencia registrada: {} → {}", db_dep.origen, db_dep.destino);

    Ok((
        StatusCode::CREATED,
        Json(DependenciaDto {
            id: db_dep.id.to_string(),
            origen: db_dep.origen,
            destino: db_dep.destino,
            descripcion: db_dep.descripcion,
            creado_en: db_dep.creado_en,
        }),
    ))
}

// ─── DELETE /deps/:origen/:destino ────────────────────────────────────────────
pub async fn eliminar_dependencia(
    State(state): State<AppState>,
    axum::extract::Path((origen, destino)): axum::extract::Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let origen = origen.trim();
    let destino = destino.trim();

    if origen.is_empty() || destino.is_empty() {
        return Err(AppError::bad_request("El origen y destino no pueden estar vacíos."));
    }

    let eliminado = db::eliminar_dependencia(&state.db, origen, destino).await?;
    if !eliminado {
        return Err(AppError::not_found("Dependencia no encontrada."));
    }

    // Actualizar motor en memoria
    {
        let mut m = state.motor.write().await;
        m.remover_dependencia(origen, destino);
    }

    tracing::info!("Dependencia eliminada: {} → {}", origen, destino);

    Ok(StatusCode::NO_CONTENT)
}

// ─── GET /deps ────────────────────────────────────────────────────────────────
/// Lista todas las dependencias con información de los servicios involucrados.
pub async fn listar_dependencias(
    State(state): State<AppState>,
) -> Result<Json<Vec<FilaVistaGrafo>>, AppError> {
    let db_filas = db::listar_dependencias_vista(&state.db).await?;

    let lista: Vec<FilaVistaGrafo> = db_filas
        .into_iter()
        .map(|f| FilaVistaGrafo {
            dep_id: f.dep_id.to_string(),
            origen: f.origen,
            desc_origen: f.desc_origen,
            destino: f.destino,
            desc_destino: f.desc_destino,
            desc_dependencia: f.desc_dependencia,
            creado_en: f.creado_en,
        })
        .collect();

    Ok(Json(lista))
}

// ─── GET /analyze ─────────────────────────────────────────────────────────────
/// Ejecuta DFS sobre el grafo en memoria, detecta ciclos y persiste el análisis.
pub async fn analizar_grafo(
    State(state): State<AppState>,
) -> Result<Json<AnalisisDto>, AppError> {
    let (tiene_ciclo, ciclos, snap) = {
        let m = state.motor.read().await;
        let tiene_ciclo = m.tiene_ciclo();
        let snap = m.instantanea();
        // Para consistencia perfecta, extraemos los ciclos concretos
        // usando el DFS pero basándonos 100% en el estado actual de petgraph.
        let mut g_temp = mesh_core::dfs::Grafo::nuevo();
        g_temp.adyacencia = snap.clone();
        let ciclos = g_temp.detectar_ciclos();
        (tiene_ciclo, ciclos, snap)
    };

    let snapshot_json = json!(snap);
    let ciclos_json = json!(ciclos);

    let (id, ejecutado_en) = db::insertar_analisis(
        &state.db,
        tiene_ciclo,
        &snapshot_json,
        &ciclos_json,
    ).await?;

    let alerta = if tiene_ciclo {
        format!(
            "⚠ ALERTA CRÍTICA: Se detectaron {} dependencia(s) circular(es).",
            ciclos.len()
        )
    } else {
        "✓ OK: No se detectaron dependencias circulares.".to_string()
    };

    tracing::info!("Análisis ejecutado. Ciclos detectados: {}", ciclos.len());

    Ok(Json(AnalisisDto {
        id: id.to_string(),
        tiene_ciclo,
        snapshot_grafo: snapshot_json,
        ciclos_detectados: ciclos_json,
        ejecutado_en,
        alerta,
    }))
}

// ─── GET /analyze/history ─────────────────────────────────────────────────────
/// Devuelve el historial de análisis ejecutados (más reciente primero).
pub async fn historial_analisis(
    State(state): State<AppState>,
) -> Result<Json<Vec<AnalisisResumen>>, AppError> {
    let db_filas = db::listar_historial_analisis(&state.db).await?;

    let lista: Vec<AnalisisResumen> = db_filas
        .into_iter()
        .map(|f| AnalisisResumen {
            id: f.id.to_string(),
            tiene_ciclo: f.tiene_ciclo,
            ejecutado_en: f.ejecutado_en,
        })
        .collect();

    Ok(Json(lista))
}

// ─── GET /analyze/ultimo ──────────────────────────────────────────────────────
/// Devuelve el último análisis ejecutado con todos sus campos.
pub async fn ultimo_analisis(
    State(state): State<AppState>,
) -> Result<Json<AnalisisDto>, AppError> {
    let f = db::obtener_ultimo_analisis(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("Todavía no se ha ejecutado ningún análisis."))?;

    let alerta = if f.tiene_ciclo {
        "⚠ ALERTA CRÍTICA: Se detectaron dependencias circulares.".to_string()
    } else {
        "✓ OK: No se detectaron dependencias circulares.".to_string()
    };

    Ok(Json(AnalisisDto {
        id: f.id.to_string(),
        tiene_ciclo: f.tiene_ciclo,
        snapshot_grafo: f.snapshot_grafo,
        ciclos_detectados: f.ciclos_detectados,
        ejecutado_en: f.ejecutado_en,
        alerta,
    }))
}