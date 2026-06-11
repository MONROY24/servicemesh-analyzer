/// Motor de grafo para el analizador de service mesh.
/// Encapsula un grafo dirigido de petgraph y el mapeo de nombres de servicio a índices de nodo.

use std::collections::HashMap;
use petgraph::algo;
use petgraph::graph::{DiGraph, NodeIndex};

/// Motor de grafo que gestiona los servicios y sus dependencias.
///
/// ## Diseño interno: Arena Allocation con NodeIndex
///
/// `NodeIndex` es internamente un `usize` que actúa como identificador (índice)
/// dentro de una arena de memoria administrada por petgraph. En lugar de almacenar
/// referencias directas (`&T`) a los nodos, el grafo asigna cada nodo a una posición
/// en su arena interna y entrega un `NodeIndex` liviano para referenciarlo.
///
/// Esto evita los problemas que surgirían al usar referencias `&T` en un grafo con ciclos:
///
/// - **Ownership**: Si un nodo A apuntara directamente al nodo B y viceversa (ciclo),
///   ninguno de los dos podría ser el único dueño del otro, violando las reglas de Rust.
/// - **Borrow Checker**: Referencias mutuas entre nodos (`&A` y `&B`) crearían ciclos
///   de préstamos que el compilador rechazaría, ya que Rust no permite referencias
///   circulares que podrían causar liberación de memoria en orden incorrecto.
///
/// Con `NodeIndex` (un simple entero), el grafo es el único dueño de todos los nodos,
/// y los índices son sólo números que no transfieren ni comparten ownership.
///
/// ## Arena Allocation vs HashMap y Localidad de Caché
///
/// Mientras que un `HashMap` dispersa sus elementos en el heap sin un orden contiguo
/// predecible, `petgraph` almacena todos los nodos y aristas secuencialmente utilizando vectores
/// (`Vec` como Arena Allocation). Esto provee una **excelente localidad de caché** (Cache Locality).
/// Durante los recorridos intensivos como el DFS, al cargar un nodo en las líneas de caché
/// de la CPU (L1/L2), los nodos adyacentes en memoria viajan con él. Esto reduce drásticamente
/// los costosos "cache misses", garantizando que el análisis de grafos grandes escale de forma
/// muchísimo más eficiente que si dependiéramos exclusivamente de múltiples diccionarios fragmentados.
///
/// # Ejemplo
///
/// ```
/// use core::graph_engine::GraphEngine;
///
/// // Crear un motor de grafo vacío
/// let mut motor = GraphEngine::new();
///
/// // Agregar dos servicios con dependencia circular entre ellos
/// motor.agregar_dependencia("servicio-a", "servicio-b");
/// motor.agregar_dependencia("servicio-b", "servicio-a");
///
/// // El motor debe detectar el ciclo
/// assert!(motor.tiene_ciclo());
/// ```
pub struct GraphEngine {
    /// Grafo dirigido donde cada nodo es el nombre de un servicio
    /// y cada arista representa una dependencia entre servicios
    pub grafo: DiGraph<String, ()>,
    /// Mapa de nombre de servicio a su índice de nodo en el grafo
    pub indices: HashMap<String, NodeIndex>,
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphEngine {
    /// Crea un nuevo GraphEngine vacío con el grafo y el mapa de índices inicializados vacíos
    pub fn new() -> Self {
        GraphEngine {
            grafo: DiGraph::new(),
            indices: HashMap::new(),
        }
    }

    /// Agrega un servicio al grafo si no existe aún.
    /// Si el servicio ya fue registrado, retorna su índice existente.
    /// Si no existe, lo inserta como nodo en el grafo y guarda el índice en el mapa.
    pub fn agregar_servicio(&mut self, nombre: &str) -> NodeIndex {
        if let Some(&indice) = self.indices.get(nombre) {
            // El servicio ya existe, retornamos su índice
            indice
        } else {
            // El servicio es nuevo, lo agregamos al grafo y guardamos el índice
            let indice = self.grafo.add_node(nombre.to_string());
            self.indices.insert(nombre.to_string(), indice);
            indice
        }
    }

    /// Remueve un servicio del grafo por completo, eliminando nodos y aristas incidentes.
    pub fn remover_servicio(&mut self, nombre: &str) {
        if let Some(indice) = self.indices.remove(nombre) {
            // Verificamos si el nodo a remover es el último antes de mutar el grafo.
            let es_ultimo = indice.index() == self.grafo.node_count() - 1;
            
            self.grafo.remove_node(indice);
            
            // Al remover un nodo, petgraph intercambia el nodo removido con el último nodo del grafo.
            // Si el nodo removido no era el último, el último nodo fue movido a su lugar y necesitamos actualizar su índice.
            if !es_ultimo {
                let nombre_movido = self.grafo[indice].clone();
                self.indices.insert(nombre_movido, indice);
            }
        }
    }

    /// Agrega una dependencia (arista dirigida) entre dos servicios.
    /// Primero asegura que ambos servicios existan como nodos en el grafo.
    /// Luego agrega la arista solo si aún no existe entre los dos nodos.
    pub fn agregar_dependencia(&mut self, desde: &str, hacia: &str) {
        // Asegurar que ambos nodos existen en el grafo
        let indice_desde = self.agregar_servicio(desde);
        let indice_hacia = self.agregar_servicio(hacia);

        // Agregar la arista solo si no existe ya entre los dos nodos
        if !self.grafo.contains_edge(indice_desde, indice_hacia) {
            self.grafo.add_edge(indice_desde, indice_hacia, ());
        }
    }

    /// Elimina una dependencia (arista dirigida) entre dos servicios, si existe.
    pub fn remover_dependencia(&mut self, desde: &str, hacia: &str) {
        if let (Some(&indice_desde), Some(&indice_hacia)) = (self.indices.get(desde), self.indices.get(hacia)) {
            if let Some(arista) = self.grafo.find_edge(indice_desde, indice_hacia) {
                self.grafo.remove_edge(arista);
            }
        }
    }

    /// Determina si el grafo contiene al menos un ciclo.
    /// Usa el algoritmo is_cyclic_directed de petgraph.
    pub fn tiene_ciclo(&self) -> bool {
        algo::is_cyclic_directed(&self.grafo)
    }

    /// Retorna el número de nodos (servicios) en el grafo.
    /// Delega al método node_count de petgraph.
    pub fn conteo_nodos(&self) -> usize {
        self.grafo.node_count()
    }

    /// Retorna el número de aristas (dependencias) en el grafo.
    /// Delega al método edge_count de petgraph.
    pub fn conteo_aristas(&self) -> usize {
        self.grafo.edge_count()
    }

    /// Genera una instantánea del grafo como mapa de adyacencia.
    /// Para cada nodo (servicio), retorna la lista de nombres de sus vecinos (dependencias directas).
    pub fn instantanea(&self) -> HashMap<String, Vec<String>> {
        let mut mapa_adyacencia: HashMap<String, Vec<String>> = HashMap::new();

        // Iterar sobre todos los nodos del grafo
        for indice_nodo in self.grafo.node_indices() {
            let nombre_nodo = self.grafo[indice_nodo].clone();

            // Recolectar los nombres de los vecinos (nodos destino de las aristas salientes)
            let vecinos: Vec<String> = self.grafo
                .neighbors(indice_nodo)
                .map(|indice_vecino| self.grafo[indice_vecino].clone())
                .collect();

            mapa_adyacencia.insert(nombre_nodo, vecinos);
        }

        mapa_adyacencia
    }
}

#[cfg(test)]
mod pruebas {
    use super::GraphEngine;

    /// Verifica que agregar_servicio dos veces con el mismo nombre no duplica nodos en el grafo
    #[test]
    fn sin_duplicacion_de_nodos() {
        let mut motor = GraphEngine::new();
        motor.agregar_servicio("servicio-a");
        motor.agregar_servicio("servicio-a"); // segunda llamada, no debe duplicar
        assert_eq!(motor.conteo_nodos(), 1);
    }

    /// Verifica que tiene_ciclo retorna true cuando existe una dependencia circular entre dos servicios
    #[test]
    fn detecta_ciclo_en_grafo_circular() {
        let mut motor = GraphEngine::new();
        motor.agregar_dependencia("servicio-a", "servicio-b");
        motor.agregar_dependencia("servicio-b", "servicio-a"); // ciclo: a -> b -> a
        assert!(motor.tiene_ciclo());
    }

    /// Verifica que tiene_ciclo retorna false en un grafo dirigido acíclico
    #[test]
    fn no_detecta_ciclo_en_grafo_aciclico() {
        let mut motor = GraphEngine::new();
        motor.agregar_dependencia("servicio-a", "servicio-b");
        motor.agregar_dependencia("servicio-b", "servicio-c"); // cadena simple sin ciclo
        assert!(!motor.tiene_ciclo());
    }

    #[test]
    fn test_remove_last_node() {
        let mut motor = GraphEngine::new();
        motor.agregar_servicio("A");
        motor.agregar_servicio("B");
        motor.remover_servicio("B");
        assert_eq!(motor.indices.get("A").unwrap().index(), 0);
        assert!(motor.indices.get("B").is_none());
        assert_eq!(motor.conteo_nodos(), 1);
    }
}
