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
pub async fn registrar_servicio(
    State(state): State<AppState>,
    Json(payload): Json<NuevoServicio>,
) -> Result<(StatusCode, Json<ServicioDto>), AppError> {
    let nombre = payload.nombre.trim().to_string();
    if nombre.is_empty() {
        return Err(AppError::bad_request("El nombre del servicio no puede estar vacío."));
    }

    let db_svc = db::insertar_servicio(&state.db, &nombre, &payload.descripcion)
        .await?
        .ok_or_else(|| AppError::conflict("Ya existe un servicio con ese nombre."))?;

    // Actualizar grafo en memoria
    {
        let mut g = state.grafo.write().await;
        g.agregar_servicio(&db_svc.nombre);
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
pub async fn registrar_dependencia(
    State(state): State<AppState>,
    Json(payload): Json<NuevaDependencia>,
) -> Result<(StatusCode, Json<DependenciaDto>), AppError> {
    if payload.origen.trim() == payload.destino.trim() {
        return Err(AppError::bad_request("Un servicio no puede depender de sí mismo (self-loop)."));
    }

    let mut tx = state.db.begin().await.map_err(|e| AppError::internal(e.to_string()))?;

    let origen = payload.origen.trim();
    let destino = payload.destino.trim();

    let db_dep = db::insertar_dependencia(&mut tx, origen, destino, &payload.descripcion)
        .await?
        .ok_or_else(|| AppError::conflict("Ya existe esa dependencia."))?;

    // Actualizar grafo en memoria
    {
        let mut g = state.grafo.write().await;
        g.agregar_dependencia(&db_dep.origen, &db_dep.destino);
        let mut m = state.motor.write().await;
        m.agregar_dependencia(&db_dep.origen, &db_dep.destino);
    }

    if let Err(e) = tx.commit().await {
        // Rollback del grafo y motor si la query falla al confirmar la transacción
        let mut g = state.grafo.write().await;
        g.remover_dependencia(&db_dep.origen, &db_dep.destino);
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
        let g = state.grafo.read().await;
        let m = state.motor.read().await;
        let ciclos = g.detectar_ciclos();
        let tiene_ciclo = m.tiene_ciclo();
        let snap = m.instantanea();
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