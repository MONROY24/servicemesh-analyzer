use sqlx::{PgPool, Postgres, Row, Transaction};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct DbServicio {
    pub id: Uuid,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
    pub creado_en: OffsetDateTime,
    pub actualizado_en: OffsetDateTime,
}

pub struct DbServicioResumen {
    pub nombre: String,
    pub descripcion: Option<String>,
}

pub struct DbDependencia {
    pub id: Uuid,
    pub origen: String,
    pub destino: String,
    pub descripcion: Option<String>,
    pub creado_en: OffsetDateTime,
}

pub struct DbVistaGrafo {
    pub dep_id: Uuid,
    pub origen: String,
    pub desc_origen: Option<String>,
    pub destino: String,
    pub desc_destino: Option<String>,
    pub desc_dependencia: Option<String>,
    pub creado_en: OffsetDateTime,
}

pub struct DbAnalisis {
    pub id: Uuid,
    pub tiene_ciclo: bool,
    pub snapshot_grafo: Value,
    pub ciclos_detectados: Value,
    pub ejecutado_en: OffsetDateTime,
}

pub struct DbAnalisisResumen {
    pub id: Uuid,
    pub tiene_ciclo: bool,
    pub ejecutado_en: OffsetDateTime,
}

pub async fn cargar_nombres_servicios_activos(db: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let servicios = sqlx::query("SELECT nombre FROM servicios WHERE activo = TRUE")
        .fetch_all(db)
        .await?;
    
    let mut nombres = Vec::new();
    for s in servicios {
        nombres.push(s.try_get("nombre")?);
    }
    Ok(nombres)
}

pub async fn cargar_pares_dependencias(db: &PgPool) -> Result<Vec<(String, String)>, sqlx::Error> {
    let dependencias = sqlx::query("SELECT origen, destino FROM dependencias")
        .fetch_all(db)
        .await?;
    
    let mut pares = Vec::new();
    for d in dependencias {
        pares.push((d.try_get("origen")?, d.try_get("destino")?));
    }
    Ok(pares)
}

pub async fn insertar_servicio(
    db: &PgPool,
    nombre: &str,
    descripcion: &Option<String>,
) -> Result<Option<DbServicio>, sqlx::Error> {
    let fila = sqlx::query(
        "INSERT INTO servicios (nombre, descripcion)
         VALUES ($1, $2)
         ON CONFLICT (nombre) DO NOTHING
         RETURNING id, nombre, descripcion, activo, creado_en, actualizado_en",
    )
    .bind(nombre)
    .bind(descripcion)
    .fetch_optional(db)
    .await?;

    if let Some(f) = fila {
        Ok(Some(DbServicio {
            id: f.try_get("id")?,
            nombre: f.try_get("nombre")?,
            descripcion: f.try_get("descripcion")?,
            activo: f.try_get("activo")?,
            creado_en: f.try_get("creado_en")?,
            actualizado_en: f.try_get("actualizado_en")?,
        }))
    } else {
        Ok(None)
    }
}

pub async fn listar_servicios_activos(db: &PgPool) -> Result<Vec<DbServicio>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT id, nombre, descripcion, activo, creado_en, actualizado_en
         FROM servicios
         WHERE activo = TRUE
         ORDER BY nombre",
    )
    .fetch_all(db)
    .await?;

    let mut servicios = Vec::new();
    for f in filas {
        servicios.push(DbServicio {
            id: f.try_get("id")?,
            nombre: f.try_get("nombre")?,
            descripcion: f.try_get("descripcion")?,
            activo: f.try_get("activo")?,
            creado_en: f.try_get("creado_en")?,
            actualizado_en: f.try_get("actualizado_en")?,
        });
    }
    Ok(servicios)
}

pub async fn listar_servicios_raiz(db: &PgPool) -> Result<Vec<DbServicioResumen>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT nombre, descripcion FROM vista_servicios_raiz ORDER BY nombre",
    )
    .fetch_all(db)
    .await?;

    let mut lista = Vec::new();
    for f in filas {
        lista.push(DbServicioResumen {
            nombre: f.try_get("nombre")?,
            descripcion: f.try_get("descripcion")?,
        });
    }
    Ok(lista)
}

pub async fn listar_servicios_hoja(db: &PgPool) -> Result<Vec<DbServicioResumen>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT nombre, descripcion FROM vista_servicios_hoja ORDER BY nombre",
    )
    .fetch_all(db)
    .await?;

    let mut lista = Vec::new();
    for f in filas {
        lista.push(DbServicioResumen {
            nombre: f.try_get("nombre")?,
            descripcion: f.try_get("descripcion")?,
        });
    }
    Ok(lista)
}

pub async fn insertar_dependencia(
    tx: &mut Transaction<'_, Postgres>,
    origen: &str,
    destino: &str,
    descripcion: &Option<String>,
) -> Result<Option<DbDependencia>, sqlx::Error> {
    let fila = sqlx::query(
        "INSERT INTO dependencias (origen, destino, descripcion)
         VALUES ($1, $2, $3)
         ON CONFLICT (origen, destino) DO NOTHING
         RETURNING id, origen, destino, descripcion, creado_en",
    )
    .bind(origen)
    .bind(destino)
    .bind(descripcion)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(f) = fila {
        Ok(Some(DbDependencia {
            id: f.try_get("id")?,
            origen: f.try_get("origen")?,
            destino: f.try_get("destino")?,
            descripcion: f.try_get("descripcion")?,
            creado_en: f.try_get("creado_en")?,
        }))
    } else {
        Ok(None)
    }
}

pub async fn listar_dependencias_vista(db: &PgPool) -> Result<Vec<DbVistaGrafo>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT dep_id, origen, desc_origen, destino, desc_destino, desc_dependencia, creado_en
         FROM vista_grafo",
    )
    .fetch_all(db)
    .await?;

    let mut lista = Vec::new();
    for f in filas {
        lista.push(DbVistaGrafo {
            dep_id: f.try_get("dep_id")?,
            origen: f.try_get("origen")?,
            desc_origen: f.try_get("desc_origen")?,
            destino: f.try_get("destino")?,
            desc_destino: f.try_get("desc_destino")?,
            desc_dependencia: f.try_get("desc_dependencia")?,
            creado_en: f.try_get("creado_en")?,
        });
    }
    Ok(lista)
}

pub async fn insertar_analisis(
    db: &PgPool,
    tiene_ciclo: bool,
    snapshot_json: &Value,
    ciclos_json: &Value,
) -> Result<(Uuid, OffsetDateTime), sqlx::Error> {
    let fila = sqlx::query(
        "INSERT INTO analisis (tiene_ciclo, snapshot_grafo, ciclos_detectados)
         VALUES ($1, $2, $3)
         RETURNING id, ejecutado_en",
    )
    .bind(tiene_ciclo)
    .bind(snapshot_json)
    .bind(ciclos_json)
    .fetch_one(db)
    .await?;

    Ok((fila.try_get("id")?, fila.try_get("ejecutado_en")?))
}

pub async fn listar_historial_analisis(db: &PgPool) -> Result<Vec<DbAnalisisResumen>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT id, tiene_ciclo, ejecutado_en
         FROM analisis
         ORDER BY ejecutado_en DESC
         LIMIT 50",
    )
    .fetch_all(db)
    .await?;

    let mut lista = Vec::new();
    for f in filas {
        lista.push(DbAnalisisResumen {
            id: f.try_get("id")?,
            tiene_ciclo: f.try_get("tiene_ciclo")?,
            ejecutado_en: f.try_get("ejecutado_en")?,
        });
    }
    Ok(lista)
}

pub async fn obtener_ultimo_analisis(db: &PgPool) -> Result<Option<DbAnalisis>, sqlx::Error> {
    let fila = sqlx::query(
        "SELECT id, tiene_ciclo, snapshot_grafo, ciclos_detectados, ejecutado_en
         FROM analisis
         ORDER BY ejecutado_en DESC
         LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    if let Some(f) = fila {
        Ok(Some(DbAnalisis {
            id: f.try_get("id")?,
            tiene_ciclo: f.try_get("tiene_ciclo")?,
            snapshot_grafo: f.try_get("snapshot_grafo")?,
            ciclos_detectados: f.try_get("ciclos_detectados")?,
            ejecutado_en: f.try_get("ejecutado_en")?,
        }))
    } else {
        Ok(None)
    }
}
