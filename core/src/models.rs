/// Modelos de datos para el analizador de service mesh

/// Representa un servicio dentro del mesh
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Service {
    /// Identificador único del servicio
    pub id: uuid::Uuid,
    /// Nombre del servicio
    pub name: String,
}

/// Representa una dependencia entre dos servicios
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// Servicio de origen de la dependencia
    pub from_service: String,
    /// Servicio de destino de la dependencia
    pub to_service: String,
}
