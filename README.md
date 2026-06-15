# ServiceMesh Analyzer

> Motor de análisis estático de dependencias entre microservicios — detecta ciclos en topologías de servicios usando **DFS** sobre grafos dirigidos, expuesto vía una **API REST** en Rust.

**Stack:** Rust · Axum · PostgreSQL · sqlx · Tokio · petgraph

---

> **Proyecto académico — Estructura de Datos 2 · GT2**  
> Universidad de El Salvador · Facultad Multidisciplinaria de Occidente

---

## ¿Qué hace este proyecto?

ServiceMesh Analyzer resuelve un problema real de arquitectura de software: detectar automáticamente cuándo dos o más microservicios se llaman entre sí de forma circular. Las dependencias circulares generan bloqueos en cascada, dificultan el despliegue independiente y son difíciles de detectar a simple vista en sistemas con decenas de servicios.

```
OrderService ──► AuthService ──► LogService
     │                ▲
     ▼                │
PaymentService ───────┘   ← ⚠ ciclo: Auth → Payment → Auth
```

Al llamar `GET /analyze`, el sistema ejecuta DFS sobre el grafo en memoria, identifica los **back-edges** que confirman ciclos y devuelve una alerta con los caminos exactos involucrados.

---

## Tabla de contenidos

- [Arquitectura](#arquitectura)
- [Análisis de complejidad](#análisis-de-complejidad)
- [Requisitos previos](#requisitos-previos)
- [Configuración rápida](#configuración-rápida)
- [Referencia de endpoints](#referencia-de-endpoints)
- [Demo: detectar un ciclo en 4 pasos](#demo-detectar-un-ciclo-en-4-pasos)
- [Pruebas](#pruebas)
- [Esquema de la base de datos](#esquema-de-la-base-de-datos)
- [Integrantes](#integrantes)

---

## Arquitectura

El proyecto usa un **Cargo Workspace** dividido en dos crates independientes que separan la lógica pura del grafo de la capa HTTP y persistencia:

```
servicemesh-analyzer/
├── Cargo.toml                  # Workspace raíz
├── .env.example                # Variables de entorno de ejemplo
├── migrations/
│   └── servicemesh_db.sql      # Esquema inicial + datos de prueba
│
├── core/                       # Crate: lógica pura del grafo
│   └── src/
│       ├── lib.rs
│       └── dfs.rs              # Grafo dirigido + detección de ciclos (DFS)
│
└── api/                        # Crate: servidor HTTP y persistencia
    └── src/
        ├── main.rs             # Punto de entrada, Tokio runtime
        ├── routes.rs           # Definición de endpoints
        ├── handlers.rs         # Lógica de cada endpoint
        ├── state.rs            # AppState: pool de BD + grafo en memoria
        └── error.rs            # Manejo centralizado de errores HTTP
```

**Stack tecnológico:**

| Capa | Tecnología |
|---|---|
| Lenguaje | Rust (edition 2024) |
| Servidor HTTP | Axum 0.8 |
| Runtime async | Tokio |
| Base de datos | PostgreSQL 15+ |
| ORM / queries | sqlx 0.8 |
| Motor de grafos | petgraph |
| Serialización | serde + serde_json |
| Logging | tracing + tracing-subscriber |

---

## Análisis de complejidad

### Detección de ciclos: O(V + E)

El motor implementa DFS con **coloreo de nodos** (blanco / gris / negro) sobre una **Lista de Adyacencia**. Cada vértice y cada arista se visitan a lo sumo una vez, logrando tiempo asintótico óptimo de **O(V + E)**.

Una implementación con **Matriz de Adyacencia** obligaría al DFS a verificar todos los destinos posibles aunque no existieran aristas, degradando el rendimiento a **O(V²)**. Dado que las arquitecturas de microservicios producen grafos dispersos (E ≪ V²), la lista de adyacencia es la estructura óptima.

| Estructura | Tiempo DFS | Espacio | Apropiada para |
|---|---|---|---|
| Lista de adyacencia ✓ | **O(V + E)** | O(V + E) | Grafos dispersos (microservicios) |
| Matriz de adyacencia | O(V²) | O(V²) | Grafos densos |

### Arena Allocation vs HashMap

En lugar de gestionar el grafo con `HashMap` anidados, el motor interno usa `NodeIndex` sobre vectores lineales (`Vec`), patrón conocido como **Arena Allocation**.

Los `HashMap` fragmentan sus elementos en el *heap*, causando altos costos en accesos secuenciales. El patrón arena almacena nodos de forma contigua en memoria, lo que garantiza **localidad de caché (L1/L2)**. Durante recorridos intensivos, al cargar un nodo en caché, los nodos vecinos se cargan colateralmente, mitigando los *cache misses* a nivel hardware.

---

## Requisitos previos

- **Rust** `1.75+` con `cargo` → [rustup.rs](https://rustup.rs)
- **Docker** (para levantar PostgreSQL) → [docs.docker.com](https://docs.docker.com/get-docker/)
- **Git**

Verifica las versiones antes de continuar:

```bash
rustc --version   # rustc 1.75.0 o superior
cargo --version
docker --version
```

---

## Configuración rápida

### 1. Clonar y configurar el entorno

```bash
git clone https://github.com/<usuario>/servicemesh-analyzer.git
cd servicemesh-analyzer

cp .env.example .env
```

Edita `.env` con tus credenciales:

```env
DATABASE_URL=postgres://postgres:TU_PASSWORD@localhost:5432/servicemesh_analyzer
```

> **Nota:** `.env` está en `.gitignore` y nunca debe subirse al repositorio.

### 2. Levantar PostgreSQL

**Opción A — Docker (recomendada):**

```bash
docker run -d \
  --name servicemesh-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=TU_PASSWORD \
  -e POSTGRES_DB=servicemesh_analyzer \
  -p 5432:5432 \
  postgres:15
```

Espera unos segundos, luego crea el esquema con datos de prueba:

```bash
docker exec -i servicemesh-db psql \
  -U postgres \
  -d servicemesh_analyzer \
  < migrations/servicemesh_db.sql
```

Salida esperada:

```
NOTICE:  ✓ Schema creado correctamente.
NOTICE:    Servicios insertados   : 7
NOTICE:    Dependencias insertadas: 9
```

**Opción B — PostgreSQL local:**

```bash
createdb servicemesh_analyzer
psql -d servicemesh_analyzer -f migrations/servicemesh_db.sql
```

### 3. Iniciar el servidor

```bash
cargo run -p api
```

```
INFO api: Conexión a PostgreSQL establecida.
INFO api: Topología cargada: 7 servicios, 9 dependencias.
INFO api: Servidor escuchando en http://0.0.0.0:3000
```

El servidor queda disponible en `http://localhost:3000`.

Para logs más detallados:

```bash
RUST_LOG=debug cargo run -p api
```

---

## Referencia de endpoints

Base URL: `http://localhost:3000`  
Todos los cuerpos y respuestas son `application/json`.

### Servicios

| Método | Ruta | Descripción |
|---|---|---|
| `POST` | `/services` | Registrar un microservicio |
| `GET` | `/services` | Listar todos los servicios activos |
| `GET` | `/services/raiz` | Servicios sin dependencias entrantes (raíces del grafo) |
| `GET` | `/services/hoja` | Servicios sin dependencias salientes (hojas del grafo) |

#### `POST /services`

```bash
curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "AuthService", "descripcion": "Autenticación de usuarios"}'
```

```json
{
  "id": "a1b2c3d4-...",
  "nombre": "AuthService",
  "descripcion": "Autenticación de usuarios",
  "activo": true,
  "creado_en": "2026-06-09 10:00:00 +00:00",
  "actualizado_en": "2026-06-09 10:00:00 +00:00"
}
```

---

### Dependencias

| Método | Ruta | Descripción |
|---|---|---|
| `POST` | `/deps` | Registrar una dependencia dirigida (`origen → destino`) |
| `GET` | `/deps` | Listar todas las dependencias |

#### `POST /deps`

```bash
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "OrderService", "destino": "AuthService", "descripcion": "Valida token antes de procesar orden"}'
```

```json
{
  "id": "e5f6g7h8-...",
  "origen": "OrderService",
  "destino": "AuthService",
  "descripcion": "Valida token antes de procesar orden",
  "creado_en": "2026-06-09 10:01:00 +00:00"
}
```

> Si `origen == destino`, la API devuelve `400 Bad Request`. Un servicio no puede depender de sí mismo.

---

### Análisis de ciclos

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/analyze` | Ejecutar análisis DFS y detectar ciclos |
| `GET` | `/analyze/history` | Historial de análisis (últimos 50) |
| `GET` | `/analyze/ultimo` | Último análisis ejecutado |

#### `GET /analyze`

Ejecuta el algoritmo DFS sobre el grafo en memoria, persiste el resultado en la tabla `analisis` y devuelve la respuesta.

```bash
curl -s http://localhost:3000/analyze
```

**Sin ciclos:**

```json
{
  "id": "x1y2z3...",
  "tiene_ciclo": false,
  "ciclos_detectados": [],
  "snapshot_grafo": {
    "AuthService": ["LogService"],
    "OrderService": ["AuthService"]
  },
  "ejecutado_en": "2026-06-09 10:05:00 +00:00",
  "alerta": "✓ OK: No se detectaron dependencias circulares."
}
```

**Con ciclo detectado:**

```json
{
  "id": "a9b8c7...",
  "tiene_ciclo": true,
  "ciclos_detectados": [
    ["AuthService", "PaymentService", "AuthService"]
  ],
  "snapshot_grafo": {
    "AuthService": ["PaymentService"],
    "PaymentService": ["AuthService"]
  },
  "ejecutado_en": "2026-06-09 10:06:00 +00:00",
  "alerta": "⚠ ALERTA CRÍTICA: Se detectaron 1 dependencia(s) circular(es)."
}
```

---

## Demo: detectar un ciclo en 4 pasos

Secuencia completa para crear un ciclo de tres servicios y verlo detectado:

```bash
# 1. Registrar los servicios
curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "AuthService"}'

curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "PaymentService"}'

curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "BillingService"}'

# 2. Crear la cadena de dependencias: Auth → Payment → Billing
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "AuthService", "destino": "PaymentService"}'

curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "PaymentService", "destino": "BillingService"}'

# 3. Cerrar el ciclo: Billing → Auth
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "BillingService", "destino": "AuthService"}'

# 4. Ejecutar el análisis
curl -s http://localhost:3000/analyze | python3 -m json.tool
```

La respuesta incluirá `"tiene_ciclo": true` y el camino exacto:

```
Auth → Payment → Billing → Auth
```

---

## Pruebas

### Unitarias — algoritmo DFS (sin base de datos)

Validan la lógica de detección de ciclos directamente sobre el crate `core`:

```bash
cargo test -p core
```

Salida esperada:

```
running 6 tests
test dfs::tests::detecta_ciclo_directo_entre_dos_servicios ... ok
test dfs::tests::detecta_ciclo_indirecto_entre_tres_servicios ... ok
test dfs::tests::no_detecta_ciclo_en_grafo_aciclico ... ok
test dfs::tests::no_detecta_ciclo_en_grafo_desconectado ... ok
test dfs::tests::no_detecta_ciclo_en_nodo_aislado ... ok
test dfs::tests::evita_dependencias_duplicadas ... ok

test result: ok. 6 passed; 0 failed
```

### Todo el workspace

```bash
cargo test
```

---

## Esquema de la base de datos

```
servicios
├── id              UUID            PK
├── nombre          VARCHAR(100)    UNIQUE NOT NULL
├── descripcion     TEXT
├── activo          BOOLEAN         DEFAULT TRUE
├── creado_en       TIMESTAMPTZ
└── actualizado_en  TIMESTAMPTZ     (auto-actualizado por trigger)

dependencias
├── id              UUID            PK
├── origen          VARCHAR(100)    FK → servicios.nombre
├── destino         VARCHAR(100)    FK → servicios.nombre
├── descripcion     TEXT
└── creado_en       TIMESTAMPTZ

analisis
├── id                  UUID        PK
├── tiene_ciclo         BOOLEAN
├── snapshot_grafo      JSONB
├── ciclos_detectados   JSONB
└── ejecutado_en        TIMESTAMPTZ
```

**Vistas disponibles:**

| Vista | Descripción |
|---|---|
| `vista_grafo` | Join de dependencias con nombres y descripciones de servicios |
| `vista_servicios_raiz` | Servicios sin dependencias entrantes |
| `vista_servicios_hoja` | Servicios sin dependencias salientes |
| `vista_ultimo_analisis` | Último registro de la tabla `analisis` |

---

## Integrantes

| Nombre | Carné | Responsabilidad principal |
|---|---|---|
| Monroy Rodríguez, Melvin José | MR24075 | 
| Escobar Arriaga, Josué Giovany | EA24012 | 
| Palma Rodriguez, Carlos Benito | PR24039 | 
| Polanco Vega, Bryan Moisés | PV21034 |
| Quinteros Rivas, Jonathan Steven | QR24001 |

---

> **Asignatura:** Estructura de Datos 2 · **Docente:** Ing. William Zamora · **Grupo:** GT2  
> Universidad de El Salvador — Facultad Multidisciplinaria de Occidente
