use std::collections::HashMap;

#[derive(Debug, PartialEq)]
enum Estado {
    NoVisitado,
    EnProgreso,
    Terminado,
}

pub struct Grafo {
    pub adyacencia: HashMap<String, Vec<String>>,
}

impl Grafo {
    pub fn nuevo() -> Self {
        Grafo {
            adyacencia: HashMap::new(),
        }
    }

    pub fn agregar_servicio(&mut self, nombre: &str) {
        self.adyacencia
            .entry(nombre.to_string())
            .or_insert_with(Vec::new);
    }

    pub fn agregar_dependencia(&mut self, origen: &str, destino: &str) {
        self.adyacencia
            .entry(origen.to_string())
            .or_insert_with(Vec::new)
            .push(destino.to_string());
    }

    /// Devuelve true si existe al menos un ciclo en el grafo.
    pub fn tiene_ciclo(&self) -> bool {
        !self.detectar_ciclos().is_empty()
    }

    /// Devuelve todos los ciclos encontrados, cada uno como lista de nodos.
    /// Usa DFS con coloreo: blanco → gris → negro.
    pub fn detectar_ciclos(&self) -> Vec<Vec<String>> {
        let mut estado: HashMap<String, Estado> = self
            .adyacencia
            .keys()
            .map(|k| (k.clone(), Estado::NoVisitado))
            .collect();

        let mut ciclos: Vec<Vec<String>> = Vec::new();
        let mut pila: Vec<String> = Vec::new();

        for nodo in self.adyacencia.keys() {
            if estado[nodo] == Estado::NoVisitado {
                self.dfs_ciclos(nodo, &mut estado, &mut pila, &mut ciclos);
            }
        }

        ciclos
    }

    fn dfs_ciclos(
        &self,
        nodo: &str,
        estado: &mut HashMap<String, Estado>,
        pila: &mut Vec<String>,
        ciclos: &mut Vec<Vec<String>>,
    ) {
        estado.insert(nodo.to_string(), Estado::EnProgreso);
        pila.push(nodo.to_string());

        if let Some(vecinos) = self.adyacencia.get(nodo) {
            for vecino in vecinos {
                match estado.get(vecino.as_str()) {
                    Some(Estado::EnProgreso) => {
                        // Encontramos un back-edge: reconstruir el ciclo desde la pila
                        if let Some(inicio) = pila.iter().position(|n| n == vecino) {
                            let ciclo: Vec<String> = pila[inicio..].to_vec();
                            ciclos.push(ciclo);
                        }
                    }
                    Some(Estado::NoVisitado) => {
                        self.dfs_ciclos(vecino, estado, pila, ciclos);
                    }
                    _ => {}
                }
            }
        }

        pila.pop();
        estado.insert(nodo.to_string(), Estado::Terminado);
    }

    /// Serializa la adyacencia del grafo como HashMap clonado,
    /// útil para generar snapshot_grafo en formato JSON.
    pub fn snapshot(&self) -> HashMap<String, Vec<String>> {
        self.adyacencia.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caso1_ciclo_directo_entre_dos_servicios() {
        let mut grafo = Grafo::nuevo();
        grafo.agregar_servicio("AuthService");
        grafo.agregar_servicio("PaymentService");
        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "AuthService");

        assert_eq!(grafo.tiene_ciclo(), true);
        assert!(!grafo.detectar_ciclos().is_empty());
    }

    #[test]
    fn caso2_ciclo_indirecto_entre_tres_servicios() {
        let mut grafo = Grafo::nuevo();
        grafo.agregar_servicio("AuthService");
        grafo.agregar_servicio("PaymentService");
        grafo.agregar_servicio("BillingService");
        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "BillingService");
        grafo.agregar_dependencia("BillingService", "AuthService");

        assert_eq!(grafo.tiene_ciclo(), true);
    }

    #[test]
    fn caso3_grafo_sin_ciclos() {
        let mut grafo = Grafo::nuevo();
        grafo.agregar_servicio("AuthService");
        grafo.agregar_servicio("PaymentService");
        grafo.agregar_servicio("BillingService");
        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("PaymentService", "BillingService");

        assert_eq!(grafo.tiene_ciclo(), false);
        assert!(grafo.detectar_ciclos().is_empty());
    }

    #[test]
    fn caso4_grafo_desconectado_sin_ciclos() {
        let mut grafo = Grafo::nuevo();
        grafo.agregar_servicio("AuthService");
        grafo.agregar_servicio("PaymentService");
        grafo.agregar_servicio("NotificationService");
        grafo.agregar_servicio("LogService");
        grafo.agregar_dependencia("AuthService", "PaymentService");
        grafo.agregar_dependencia("NotificationService", "LogService");

        assert_eq!(grafo.tiene_ciclo(), false);
    }

    #[test]
    fn caso5_nodo_aislado_sin_dependencias() {
        let mut grafo = Grafo::nuevo();
        grafo.agregar_servicio("MonitoringService");

        assert_eq!(grafo.tiene_ciclo(), false);
    }
}