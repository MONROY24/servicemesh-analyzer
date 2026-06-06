use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use time::OffsetDateTime;

use crate::state::AppState;
use crate::error::AppError;

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

    let fila = sqlx::query(
        "INSERT INTO servicios (nombre, descripcion)
         VALUES ($1, $2)
         ON CONFLICT (nombre) DO NOTHING
         RETURNING id, nombre, descripcion, activo, creado_en, actualizado_en",
    )
    .bind(&nombre)
    .bind(&payload.descripcion)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::conflict("Ya existe un servicio con ese nombre."))?;

    let id: uuid::Uuid = fila.try_get("id")?;
    let nombre_db: String = fila.try_get("nombre")?;
    let descripcion: Option<String> = fila.try_get("descripcion")?;
    let activo: bool = fila.try_get("activo")?;
    let creado_en: OffsetDateTime = fila.try_get("creado_en")?;
    let actualizado_en: OffsetDateTime = fila.try_get("actualizado_en")?;

    // Actualizar grafo en memoria
    {
        let mut g = state.grafo.lock().unwrap();
        g.agregar_servicio(&nombre_db);
    }

    tracing::info!("Servicio registrado: {}", nombre_db);

    Ok((
        StatusCode::CREATED,
        Json(ServicioDto {
            id: id.to_string(),
            nombre: nombre_db,
            descripcion,
            activo,
            creado_en,
            actualizado_en,
        }),
    ))
}

// ─── GET /services ────────────────────────────────────────────────────────────
/// Lista todos los servicios activos.
pub async fn listar_servicios(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioDto>>, AppError> {
    let filas = sqlx::query(
        "SELECT id, nombre, descripcion, activo, creado_en, actualizado_en
         FROM servicios
         WHERE activo = TRUE
         ORDER BY nombre",
    )
    .fetch_all(&state.db)
    .await?;

    let servicios: Result<Vec<ServicioDto>, sqlx::Error> = filas
        .iter()
        .map(|f| {
            Ok(ServicioDto {
                id: f.try_get::<uuid::Uuid, _>("id")?.to_string(),
                nombre: f.try_get("nombre")?,
                descripcion: f.try_get("descripcion")?,
                activo: f.try_get("activo")?,
                creado_en: f.try_get("creado_en")?,
                actualizado_en: f.try_get("actualizado_en")?,
            })
        })
        .collect();

    Ok(Json(servicios?))
}

// ─── GET /services/raiz ───────────────────────────────────────────────────────
/// Servicios activos que no tienen ninguna dependencia entrante (raíces del grafo).
pub async fn servicios_raiz(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioResumen>>, AppError> {
    let filas = sqlx::query(
        "SELECT nombre, descripcion FROM vista_servicios_raiz ORDER BY nombre",
    )
    .fetch_all(&state.db)
    .await?;

    let lista: Result<Vec<ServicioResumen>, sqlx::Error> = filas
        .iter()
        .map(|f| {
            Ok(ServicioResumen {
                nombre: f.try_get("nombre")?,
                descripcion: f.try_get("descripcion")?,
            })
        })
        .collect();

    Ok(Json(lista?))
}

// ─── GET /services/hoja ───────────────────────────────────────────────────────
/// Servicios activos sin dependencias salientes (hojas del grafo).
pub async fn servicios_hoja(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServicioResumen>>, AppError> {
    let filas = sqlx::query(
        "SELECT nombre, descripcion FROM vista_servicios_hoja ORDER BY nombre",
    )
    .fetch_all(&state.db)
    .await?;

    let lista: Result<Vec<ServicioResumen>, sqlx::Error> = filas
        .iter()
        .map(|f| {
            Ok(ServicioResumen {
                nombre: f.try_get("nombre")?,
                descripcion: f.try_get("descripcion")?,
            })
        })
        .collect();

    Ok(Json(lista?))
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

    let fila = sqlx::query(
        "INSERT INTO dependencias (origen, destino, descripcion)
         VALUES ($1, $2, $3)
         ON CONFLICT (origen, destino) DO NOTHING
         RETURNING id, origen, destino, descripcion, creado_en",
    )
    .bind(payload.origen.trim())
    .bind(payload.destino.trim())
    .bind(&payload.descripcion)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::conflict("Ya existe esa dependencia."))?;

    let id: uuid::Uuid = fila.try_get("id")?;
    let origen: String = fila.try_get("origen")?;
    let destino: String = fila.try_get("destino")?;
    let descripcion: Option<String> = fila.try_get("descripcion")?;
    let creado_en: OffsetDateTime = fila.try_get("creado_en")?;

    // Actualizar grafo en memoria
    {
        let mut g = state.grafo.lock().unwrap();
        g.agregar_dependencia(&origen, &destino);
    }

    tracing::info!("Dependencia registrada: {} → {}", origen, destino);

    Ok((
        StatusCode::CREATED,
        Json(DependenciaDto {
            id: id.to_string(),
            origen,
            destino,
            descripcion,
            creado_en,
        }),
    ))
}

// ─── GET /deps ────────────────────────────────────────────────────────────────
/// Lista todas las dependencias con información de los servicios involucrados.
pub async fn listar_dependencias(
    State(state): State<AppState>,
) -> Result<Json<Vec<FilaVistaGrafo>>, AppError> {
    let filas = sqlx::query(
        "SELECT dep_id, origen, desc_origen, destino, desc_destino, desc_dependencia, creado_en
         FROM vista_grafo",
    )
    .fetch_all(&state.db)
    .await?;

    let lista: Result<Vec<FilaVistaGrafo>, sqlx::Error> = filas
        .iter()
        .map(|f| {
            Ok(FilaVistaGrafo {
                dep_id: f.try_get::<uuid::Uuid, _>("dep_id")?.to_string(),
                origen: f.try_get("origen")?,
                desc_origen: f.try_get("desc_origen")?,
                destino: f.try_get("destino")?,
                desc_destino: f.try_get("desc_destino")?,
                desc_dependencia: f.try_get("desc_dependencia")?,
                creado_en: f.try_get("creado_en")?,
            })
        })
        .collect();

    Ok(Json(lista?))
}

// ─── GET /analyze ─────────────────────────────────────────────────────────────
/// Ejecuta DFS sobre el grafo en memoria, detecta ciclos y persiste el análisis.
pub async fn analizar_grafo(
    State(state): State<AppState>,
) -> Result<Json<AnalisisDto>, AppError> {
    let (tiene_ciclo, ciclos, snap) = {
        let g = state.grafo.lock().unwrap();
        let ciclos = g.detectar_ciclos();
        let tiene_ciclo = !ciclos.is_empty();
        let snap = g.snapshot();
        (tiene_ciclo, ciclos, snap)
    };

    let snapshot_json = json!(snap);
    let ciclos_json = json!(ciclos);

    // Persistir en tabla analisis
    let fila = sqlx::query(
        "INSERT INTO analisis (tiene_ciclo, snapshot_grafo, ciclos_detectados)
         VALUES ($1, $2, $3)
         RETURNING id, ejecutado_en",
    )
    .bind(tiene_ciclo)
    .bind(&snapshot_json)
    .bind(&ciclos_json)
    .fetch_one(&state.db)
    .await?;

    let id: uuid::Uuid = fila.try_get("id")?;
    let ejecutado_en: OffsetDateTime = fila.try_get("ejecutado_en")?;

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
    let filas = sqlx::query(
        "SELECT id, tiene_ciclo, ejecutado_en
         FROM analisis
         ORDER BY ejecutado_en DESC
         LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?;

    let lista: Result<Vec<AnalisisResumen>, sqlx::Error> = filas
        .iter()
        .map(|f| {
            Ok(AnalisisResumen {
                id: f.try_get::<uuid::Uuid, _>("id")?.to_string(),
                tiene_ciclo: f.try_get("tiene_ciclo")?,
                ejecutado_en: f.try_get("ejecutado_en")?,
            })
        })
        .collect();

    Ok(Json(lista?))
}

// ─── GET /analyze/ultimo ──────────────────────────────────────────────────────
/// Devuelve el último análisis ejecutado con todos sus campos.
pub async fn ultimo_analisis(
    State(state): State<AppState>,
) -> Result<Json<AnalisisDto>, AppError> {
    let fila = sqlx::query(
        "SELECT id, tiene_ciclo, snapshot_grafo, ciclos_detectados, ejecutado_en
         FROM analisis
         ORDER BY ejecutado_en DESC
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Todavía no se ha ejecutado ningún análisis."))?;

    let id: uuid::Uuid = fila.try_get("id")?;
    let tiene_ciclo: bool = fila.try_get("tiene_ciclo")?;
    let snapshot_grafo: Value = fila.try_get("snapshot_grafo")?;
    let ciclos_detectados: Value = fila.try_get("ciclos_detectados")?;
    let ejecutado_en: OffsetDateTime = fila.try_get("ejecutado_en")?;

    let alerta = if tiene_ciclo {
        "⚠ ALERTA CRÍTICA: Se detectaron dependencias circulares.".to_string()
    } else {
        "✓ OK: No se detectaron dependencias circulares.".to_string()
    };

    Ok(Json(AnalisisDto {
        id: id.to_string(),
        tiene_ciclo,
        snapshot_grafo,
        ciclos_detectados,
        ejecutado_en,
        alerta,
    }))
}