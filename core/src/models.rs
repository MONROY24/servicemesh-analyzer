/// Modelos de datos para el analizador de service mesh

/// Representa un servicio dentro del mesh
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Service {
    /// Identificador único del servicio
    pub id: uuid::Uuid,
    /// Nombre del servicio
    pub name: String,
}

impl Service {
    pub fn new(id: uuid::Uuid, name: String) -> Result<Self, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre del servicio no puede estar vacío".to_string());
        }
        Ok(Self { id, name: name.to_string() })
    }
}

/// Representa una dependencia entre dos servicios
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// Servicio de origen de la dependencia
    pub from_service: String,
    /// Servicio de destino de la dependencia
    pub to_service: String,
}

impl Dependency {
    pub fn new(from_service: String, to_service: String) -> Result<Self, String> {
        let from = from_service.trim();
        let to = to_service.trim();

        if from.is_empty() || to.is_empty() {
            return Err("Los nombres de los servicios origen y destino no pueden estar vacíos".to_string());
        }

        if from == to {
            return Err("Un servicio no puede depender de sí mismo".to_string());
        }

        Ok(Self {
            from_service: from.to_string(),
            to_service: to.to_string(),
        })
    }
}
