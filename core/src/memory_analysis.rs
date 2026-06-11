//! Demostración académica del reto de memoria en Rust al modelar grafos cíclicos.
//!
//! Este módulo forma parte del análisis técnico del proyecto ServiceMesh Analyzer.
//! Su objetivo es demostrar por qué Rust rechaza grafos construidos con referencias
//! directas `&T` y por qué una representación basada en índices `usize` es más adecuada.

/// =======================================================
/// EJEMPLO 1: Grafo cíclico con referencias &T
/// =======================================================
///
/// Este código NO debe ejecutarse dentro del proyecto porque no compila.
/// Se deja comentado como evidencia técnica para el Anexo A.
///
/// Representa:
///
/// Servicio A -> Servicio B
/// Servicio B -> Servicio A
///
/// ```compile_fail
/// struct Servicio<'a> {
///     nombre: String,
///     dependencia: Option<&'a Servicio<'a>>,
/// }
///
/// fn main() {
///     let mut servicio_a = Servicio {
///         nombre: String::from("Servicio A"),
///         dependencia: None,
///     };
///
///     let mut servicio_b = Servicio {
///         nombre: String::from("Servicio B"),
///         dependencia: None,
///     };
///
///     servicio_a.dependencia = Some(&servicio_b);
///
///     // Error esperado:
///     // error[E0506]: cannot assign to `servicio_b.dependencia`
///     // because it is borrowed
///     servicio_b.dependencia = Some(&servicio_a);
///
///     println!("{}", servicio_a.nombre);
/// }
/// ```
///
/// El error ocurre porque `servicio_b` queda prestado cuando
/// `servicio_a.dependencia` almacena una referencia hacia él.
/// Luego se intenta modificar `servicio_b.dependencia`, pero Rust
/// no permite modificar un valor mientras existe una referencia activa.

#[derive(Debug, Clone)]
pub struct ServicioIndice {
    pub nombre: String,
}

/// Grafo simplificado basado en índices `usize`.
///
/// Este enfoque evita referencias directas entre nodos.
/// Es conceptualmente similar al uso de `NodeIndex` en `petgraph`.
#[derive(Debug, Clone, Default)]
pub struct GrafoConIndices {
    pub servicios: Vec<ServicioIndice>,
    pub dependencias: Vec<(usize, usize)>,
}

impl GrafoConIndices {
    pub fn nuevo() -> Self {
        Self {
            servicios: Vec::new(),
            dependencias: Vec::new(),
        }
    }

    pub fn agregar_servicio(&mut self, nombre: &str) -> usize {
        let indice = self.servicios.len();

        self.servicios.push(ServicioIndice {
            nombre: nombre.to_string(),
        });

        indice
    }

    pub fn agregar_dependencia(&mut self, origen: usize, destino: usize) -> Result<(), String> {
        let len = self.servicios.len();
        if origen >= len || destino >= len {
            return Err(format!("Índice fuera de rango: origen={}, destino={}, max={}", origen, destino, len.saturating_sub(1)));
        }
        self.dependencias.push((origen, destino));
        Ok(())
    }

    pub fn imprimir_topologia(&self) -> Vec<String> {
        self.dependencias
            .iter()
            .map(|(origen, destino)| {
                format!(
                    "{} -> {}",
                    self.servicios[*origen].nombre,
                    self.servicios[*destino].nombre
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permite_representar_ciclo_usando_indices_usize() {
        let mut grafo = GrafoConIndices::nuevo();

        let servicio_a = grafo.agregar_servicio("Servicio A");
        let servicio_b = grafo.agregar_servicio("Servicio B");

        grafo.agregar_dependencia(servicio_a, servicio_b).unwrap();
        grafo.agregar_dependencia(servicio_b, servicio_a).unwrap();

        let topologia = grafo.imprimir_topologia();

        assert_eq!(topologia.len(), 2);
        assert!(topologia.contains(&"Servicio A -> Servicio B".to_string()));
        assert!(topologia.contains(&"Servicio B -> Servicio A".to_string()));
    }
}