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

    pub fn tiene_ciclo(&self) -> bool {
        let mut estado: HashMap<String, Estado> = HashMap::new();

        for nodo in self.adyacencia.keys() {
            estado.insert(nodo.clone(), Estado::NoVisitado);
        }

        for nodo in self.adyacencia.keys() {
            if estado[nodo] == Estado::NoVisitado {
                if self.dfs_visitar(nodo, &mut estado) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_visitar(&self, nodo: &str, estado: &mut HashMap<String, Estado>) -> bool {
        estado.insert(nodo.to_string(), Estado::EnProgreso);

        if let Some(vecinos) = self.adyacencia.get(nodo) {
            for vecino in vecinos {
                match estado.get(vecino.as_str()) {
                    Some(Estado::EnProgreso) => return true, // back-edge!
                    Some(Estado::NoVisitado) => {
                        if self.dfs_visitar(vecino, estado) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }

        estado.insert(nodo.to_string(), Estado::Terminado);
        false
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