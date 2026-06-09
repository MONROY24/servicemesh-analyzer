use std::collections::HashMap;

/// Estado de visita utilizado por el algoritmo DFS.
///
/// NoVisitado: el nodo aún no ha sido procesado.
/// EnProgreso: el nodo está dentro de la pila actual de recursión.
/// Terminado: el nodo y sus dependencias ya fueron analizados.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Estado {
    NoVisitado,
    EnProgreso,
    Terminado,
}

/// Representa un grafo dirigido de dependencias entre microservicios.
///
/// Cada clave del HashMap representa un servicio.
/// Cada vector asociado contiene los servicios de los que depende.
#[derive(Debug, Clone, Default)]
pub struct Grafo {
    pub adyacencia: HashMap<String, Vec<String>>,
}

impl Grafo {
    /// Crea una nueva instancia vacía del grafo.
    pub fn nuevo() -> Self {
        Self {
            adyacencia: HashMap::new(),
        }
    }

    /// Registra un servicio dentro del grafo.
    ///
    /// Si el servicio ya existe, no se duplica.
    pub fn agregar_servicio(&mut self, nombre: &str) {
        self.adyacencia.entry(nombre.to_string()).or_default();
    }

    /// Agrega una dependencia dirigida entre dos servicios.
    ///
    /// La relación representa:
    /// origen -> destino
    ///
    /// Esto significa que el servicio `origen` depende del servicio `destino`.
    pub fn agregar_dependencia(&mut self, origen: &str, destino: &str) {
        self.agregar_servicio(origen);
        self.agregar_servicio(destino);

        let dependencias = self.adyacencia.entry(origen.to_string()).or_default();

        if !dependencias.iter().any(|d| d == destino) {
            dependencias.push(destino.to_string());
        }
    }

    /// Indica si el grafo contiene al menos un ciclo.
    ///
    /// Esta función se utiliza como una verificación rápida para determinar
    /// si existe una dependencia circular entre microservicios.
    pub fn tiene_ciclo(&self) -> bool {
        !self.detectar_ciclos().is_empty()
    }

    /// Detecta todos los ciclos encontrados en el grafo mediante DFS.
    ///
    /// El algoritmo utiliza una estrategia de coloreo:
    ///
    /// - NoVisitado: nodo pendiente de explorar.
    /// - EnProgreso: nodo actualmente en la pila de recursión.
    /// - Terminado: nodo completamente analizado.
    ///
    /// Cuando durante el recorrido se encuentra un nodo en estado EnProgreso,
    /// se identifica un back-edge, lo cual confirma la existencia de un ciclo.
    pub fn detectar_ciclos(&self) -> Vec<Vec<String>> {
        let mut estado = self.inicializar_estados();
        let mut pila_recursion: Vec<String> = Vec::new();
        let mut ciclos_detectados: Vec<Vec<String>> = Vec::new();

        for servicio in self.adyacencia.keys() {
            if estado.get(servicio) == Some(&Estado::NoVisitado) {
                self.dfs_detectar_ciclos(
                    servicio,
                    &mut estado,
                    &mut pila_recursion,
                    &mut ciclos_detectados,
                );
            }
        }

        ciclos_detectados
    }

    /// Inicializa todos los servicios del grafo como no visitados.
    fn inicializar_estados(&self) -> HashMap<String, Estado> {
        self.adyacencia
            .keys()
            .map(|servicio| (servicio.clone(), Estado::NoVisitado))
            .collect()
    }

    /// Ejecuta DFS recursivo para detectar ciclos en el grafo.
    ///
    /// Si un vecino ya se encuentra en la pila de recursión, se detecta
    /// un back-edge y se reconstruye el ciclo correspondiente.
    fn dfs_detectar_ciclos(
        &self,
        servicio_actual: &str,
        estado: &mut HashMap<String, Estado>,
        pila_recursion: &mut Vec<String>,
        ciclos_detectados: &mut Vec<Vec<String>>,
    ) {
        estado.insert(servicio_actual.to_string(), Estado::EnProgreso);
        pila_recursion.push(servicio_actual.to_string());

        if let Some(dependencias) = self.adyacencia.get(servicio_actual) {
            for servicio_dependiente in dependencias {
                match estado.get(servicio_dependiente.as_str()) {
                    Some(Estado::NoVisitado) => {
                        self.dfs_detectar_ciclos(
                            servicio_dependiente,
                            estado,
                            pila_recursion,
                            ciclos_detectados,
                        );
                    }

                    Some(Estado::EnProgreso) => {
                        if let Some(ciclo) =
                            Self::reconstruir_ciclo(pila_recursion, servicio_dependiente)
                        {
                            ciclos_detectados.push(ciclo);
                        }
                    }

                    Some(Estado::Terminado) | None => {}
                }
            }
        }

        pila_recursion.pop();
        estado.insert(servicio_actual.to_string(), Estado::Terminado);
    }

    /// Reconstruye un ciclo a partir de la pila de recursión.
    ///
    /// Ejemplo:
    /// Pila: Auth -> Payment -> Billing
    /// Back-edge: Billing -> Auth
    /// Ciclo detectado: Auth -> Payment -> Billing -> Auth
    fn reconstruir_ciclo(
        pila_recursion: &[String],
        inicio_ciclo: &str,
    ) -> Option<Vec<String>> {
        let posicion_inicio = pila_recursion
            .iter()
            .position(|servicio| servicio == inicio_ciclo)?;

        let mut ciclo = pila_recursion[posicion_inicio..].to_vec();
        ciclo.push(inicio_ciclo.to_string());

        Some(ciclo)
    }

    /// Genera una copia del grafo en formato HashMap.
    ///
    /// Esta función es útil para serializar el estado actual del grafo
    /// como snapshot dentro del análisis del sistema.
    pub fn snapshot(&self) -> HashMap<String, Vec<String>> {
        self.adyacencia.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_ciclo_directo_entre_dos_servicios() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "AuthService");

        assert!(grafo.tiene_ciclo());
        assert!(!grafo.detectar_ciclos().is_empty());
    }

    #[test]
    fn detecta_ciclo_indirecto_entre_tres_servicios() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "BillingService");
        grafo.agregar_dependencia("BillingService", "AuthService");

        let ciclos = grafo.detectar_ciclos();

        assert!(grafo.tiene_ciclo());
        assert!(!ciclos.is_empty());

        let ciclo = &ciclos[0];

        assert!(ciclo.contains(&"AuthService".to_string()));
        assert!(ciclo.contains(&"PaymentService".to_string()));
        assert!(ciclo.contains(&"BillingService".to_string()));
        assert_eq!(ciclo.first(), ciclo.last());
    }

    #[test]
    fn no_detecta_ciclo_en_grafo_aciclico() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "BillingService");

        assert!(!grafo.tiene_ciclo());
        assert!(grafo.detectar_ciclos().is_empty());
    }

    #[test]
    fn no_detecta_ciclo_en_grafo_desconectado() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("NotificationService", "LogService");

        assert!(!grafo.tiene_ciclo());
    }

    #[test]
    fn no_detecta_ciclo_en_nodo_aislado() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_servicio("MonitoringService");

        assert!(!grafo.tiene_ciclo());
        assert!(grafo.detectar_ciclos().is_empty());
    }

    #[test]
    fn evita_dependencias_duplicadas() {
        let mut grafo = Grafo::nuevo();

        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("AuthService", "PaymentService");

        assert_eq!(grafo.adyacencia.get("AuthService").unwrap().len(), 1);
    }
}