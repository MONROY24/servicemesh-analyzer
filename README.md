# ServiceMesh Analyzer

Motor de análisis estático de dependencias entre microservicios construido en **Rust**. Modela la topología de servicios como un grafo dirigido y detecta dependencias circulares mediante **Búsqueda en Profundidad (DFS)**. Expone los resultados a través de una API REST construida con **Axum** y persiste la información en **PostgreSQL**.

> Proyecto académico — Estructura de Datos 2 · GT2  
> Universidad de El Salvador · Facultad Multidisciplinaria de Occidente

---

## Tabla de contenidos

- [Descripción del sistema](#descripción-del-sistema)
- [Arquitectura](#arquitectura)
- [Requisitos previos](#requisitos-previos)
- [Configuración del entorno](#configuración-del-entorno)
- [Levantar PostgreSQL](#levantar-postgresql)
- [Ejecutar la API](#ejecutar-la-api)
- [Ejecutar las pruebas](#ejecutar-las-pruebas)
- [Referencia de endpoints](#referencia-de-endpoints)
- [Demo rápida con ciclo](#demo-rápida-con-ciclo)
- [Esquema de la base de datos](#esquema-de-la-base-de-datos)
- [Integrantes](#integrantes)

---

## Descripción del sistema

ServiceMesh Analyzer resuelve un problema concreto de arquitectura de software: detectar automáticamente cuando dos o más microservicios se llaman entre sí de forma circular, lo que genera bloqueos en cascada y dificulta el despliegue independiente de servicios.

```
OrderService ──► AuthService ──► LogService
     │                ▲
     ▼                │
PaymentService ───────┘   ← ciclo detectado: Auth → Payment → Auth
```

Cuando se llama a `GET /analyze`, el sistema ejecuta DFS sobre el grafo en memoria, identifica los **back-edges** que confirman ciclos y devuelve una alerta arquitectónica con los caminos exactos involucrados.

---

## Análisis Teórico de Complejidad

En cumplimiento de los requerimientos de diseño algorítmico, el motor de grafos toma decisiones clave para garantizar la máxima eficiencia en la estructura subyacente:

### 1. Detección de Ciclos: O(V + E) vs O(V²)
Para detectar dependencias circulares, implementamos un recorrido en profundidad (DFS) con **coloreo de nodos**. Al representar la topología de servicios mediante una **Lista de Adyacencia** (donde cada nodo apunta solo a sus dependientes directos), el DFS visita cada vértice (V) y cada arista (E) a lo sumo una vez, logrando un tiempo asintótico óptimo de **O(V + E)**.

Si el grafo se implementara de forma ingenua como una **Matriz de Adyacencia** (una cuadrícula $V \times V$), el DFS se vería obligado a verificar cada destino posible iterativamente aunque no existieran aristas, degradando el rendimiento a un estricto e inevitable **O(V²)**. Dado que las arquitecturas de microservicios configuran grafos fuertemente dispersos (*sparse graphs*, donde $E \ll V²$), la lista de adyacencia en $O(V + E)$ ofrece un desempeño infinitamente superior.

### 2. Arena Allocation vs HashMap (Localidad de Caché)
A diferencia de los enfoques básicos que gestionan grafos usando múltiples diccionarios (`HashMap`) anidados, nuestro motor interno gestiona los identificadores mediante `NodeIndex` encapsulando vectores lineales (`Vec`), usando un patrón conocido como **Arena Allocation**.

El problema de un `HashMap` es que fragmenta dinámicamente sus elementos en el *heap*, ocasionando altos costos en accesos secuenciales. Por el contrario, el orden contiguo en memoria del patrón arena asegura una inmejorable **localidad de caché (Cache Locality)**. Durante el recorrido intensivo de grafos gigantes, al cargar un nodo en las líneas de caché L1/L2 del procesador, los nodos vecinos son cargados colateralmente, mitigando drásticamente los costosos retrasos por *cache misses* (fallos de caché) a nivel hardware.

---

## Arquitectura

El proyecto usa un **Cargo Workspace** dividido en dos crates independientes:

```
servicemesh-analyzer/
├── Cargo.toml              # Workspace raíz
├── .env.example            # Variables de entorno de ejemplo
├── migrations/
│   └── servicemesh_db.sql  # Esquema inicial de PostgreSQL
│
├── core/                   # Crate: lógica pura del grafo y algoritmos
│   └── src/
│       ├── lib.rs
│       └── dfs.rs          # Grafo dirigido + detección de ciclos (DFS)
│
└── api/                    # Crate: servidor HTTP (Axum) y persistencia
    └── src/
        ├── main.rs         # Punto de entrada, Tokio runtime
        ├── routes.rs       # Definición de endpoints
        ├── handlers.rs     # Lógica de cada endpoint
        ├── state.rs        # AppState: pool de BD + grafo en memoria
        └── error.rs        # Manejo centralizado de errores HTTP
```

**Stack tecnológico:**

| Capa | Tecnología |
|---|---|
| Lenguaje | Rust (edition 2024) |
| Servidor HTTP | Axum 0.8 |
| Runtime async | Tokio |
| Base de datos | PostgreSQL 15+ |
| ORM / queries | sqlx 0.8 |
| Serialización | serde + serde_json |
| Logging | tracing + tracing-subscriber |

---

## Requisitos previos

Asegúrate de tener instalado lo siguiente antes de continuar:

- **Rust** `1.75+` con `cargo`  
  → Instalar: https://rustup.rs

- **Docker** (para levantar PostgreSQL sin instalación local)  
  → Instalar: https://docs.docker.com/get-docker/

- **Git**

Verifica las versiones:

```bash
rustc --version   # rustc 1.75.0 o superior
cargo --version
docker --version
```

---

## Configuración del entorno

Copia el archivo de ejemplo y edita las credenciales:

```bash
cp .env.example .env
```

Abre `.env` y ajusta los valores:

```env
DATABASE_URL=postgres://postgres:TU_PASSWORD@localhost:5432/servicemesh_analyzer
```

| Variable | Descripción | Valor por defecto |
|---|---|---|
| `DATABASE_URL` | URL de conexión a PostgreSQL | `postgres://postgres:TU_PASSWORD@localhost:5432/servicemesh_analyzer` |

> **Importante:** el archivo `.env` está en `.gitignore` y nunca debe subirse al repositorio.

---

## Levantar PostgreSQL

### Opción A — Docker (recomendada)

Levanta un contenedor de PostgreSQL con las credenciales del `.env`:

```bash
docker run -d \
  --name servicemesh-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=TU_PASSWORD \
  -e POSTGRES_DB=servicemesh_analyzer \
  -p 5432:5432 \
  postgres:15
```

Espera unos segundos y luego crea el esquema:

```bash
docker exec -i servicemesh-db psql \
  -U postgres \
  -d servicemesh_analyzer \
  < migrations/servicemesh_db.sql
```

Deberías ver en la salida:

```
NOTICE:  ✓ Schema creado correctamente.
NOTICE:    Servicios insertados   : 7
NOTICE:    Dependencias insertadas: 9
```

### Opción B — PostgreSQL local

Si ya tienes PostgreSQL instalado localmente:

```bash
createdb servicemesh_analyzer
psql -d servicemesh_analyzer -f migrations/servicemesh_db.sql
```

---

## Ejecutar la API

Con la base de datos corriendo y el `.env` configurado:

```bash
cargo run -p api
```

Deberías ver:

```
INFO api: Conexión a PostgreSQL establecida.
INFO api: Topología cargada: 7 servicios, 9 dependencias.
INFO api: Servidor escuchando en http://0.0.0.0:3000
```

El servidor queda disponible en `http://localhost:3000`.

Para activar logs más detallados:

```bash
RUST_LOG=debug cargo run -p api
```

---

## Ejecutar las pruebas

### Pruebas unitarias del algoritmo DFS

No requieren base de datos. Validan la lógica de detección de ciclos directamente:

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

### Todas las pruebas del workspace

```bash
cargo test
```

---

## Referencia de endpoints

La API escucha en `http://localhost:3000`. Todos los cuerpos son `application/json`.

---

### Servicios

#### `POST /services` — Registrar un microservicio

```bash
curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "AuthService", "descripcion": "Autenticación de usuarios"}'
```

**Respuesta `201 Created`:**
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

#### `GET /services` — Listar todos los servicios activos

```bash
curl -s http://localhost:3000/services
```

---

#### `GET /services/raiz` — Servicios sin dependencias entrantes (raíces del grafo)

Servicios que no son dependencia de ningún otro.

```bash
curl -s http://localhost:3000/services/raiz
```

---

#### `GET /services/hoja` — Servicios sin dependencias salientes (hojas del grafo)

Servicios que no dependen de ningún otro.

```bash
curl -s http://localhost:3000/services/hoja
```

---

### Dependencias

#### `POST /deps` — Registrar una dependencia dirigida

`origen` depende de `destino` (origen → destino).

```bash
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "OrderService", "destino": "AuthService", "descripcion": "Valida token antes de procesar orden"}'
```

**Respuesta `201 Created`:**
```json
{
  "id": "e5f6g7h8-...",
  "origen": "OrderService",
  "destino": "AuthService",
  "descripcion": "Valida token antes de procesar orden",
  "creado_en": "2026-06-09 10:01:00 +00:00"
}
```

> **Nota:** Un servicio no puede depender de sí mismo. Si `origen == destino` la API devuelve `400 Bad Request`.

---

#### `GET /deps` — Listar todas las dependencias

```bash
curl -s http://localhost:3000/deps
```

---

### Análisis de ciclos

#### `GET /analyze` — Ejecutar análisis DFS y detectar ciclos

Ejecuta el algoritmo DFS sobre el grafo en memoria, persiste el resultado en la tabla `analisis` y devuelve la respuesta.

```bash
curl -s http://localhost:3000/analyze
```

**Respuesta sin ciclos:**
```json
{
  "id": "x1y2z3...",
  "tiene_ciclo": false,
  "ciclos_detectados": [],
  "snapshot_grafo": { "AuthService": ["LogService"], "OrderService": ["AuthService"] },
  "ejecutado_en": "2026-06-09 10:05:00 +00:00",
  "alerta": "✓ OK: No se detectaron dependencias circulares."
}
```

**Respuesta con ciclo detectado:**
```json
{
  "id": "a9b8c7...",
  "tiene_ciclo": true,
  "ciclos_detectados": [
    ["AuthService", "PaymentService", "AuthService"]
  ],
  "snapshot_grafo": { "AuthService": ["PaymentService"], "PaymentService": ["AuthService"] },
  "ejecutado_en": "2026-06-09 10:06:00 +00:00",
  "alerta": "⚠ ALERTA CRÍTICA: Se detectaron 1 dependencia(s) circular(es)."
}
```

---

#### `GET /analyze/history` — Historial de análisis (últimos 50)

```bash
curl -s http://localhost:3000/analyze/history
```

---

#### `GET /analyze/ultimo` — Último análisis ejecutado

```bash
curl -s http://localhost:3000/analyze/ultimo
```

---

## Demo rápida con ciclo

Secuencia completa para crear un ciclo y verlo detectado:

```bash
# 1. Registrar servicios
curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "AuthService"}'

curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "PaymentService"}'

curl -s -X POST http://localhost:3000/services \
  -H "Content-Type: application/json" \
  -d '{"nombre": "BillingService"}'

# 2. Crear dependencias en cadena
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "AuthService", "destino": "PaymentService"}'

curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "PaymentService", "destino": "BillingService"}'

# 3. Cerrar el ciclo: BillingService → AuthService
curl -s -X POST http://localhost:3000/deps \
  -H "Content-Type: application/json" \
  -d '{"origen": "BillingService", "destino": "AuthService"}'

# 4. Ejecutar el análisis
curl -s http://localhost:3000/analyze | python3 -m json.tool
```

La respuesta mostrará `"tiene_ciclo": true` y el camino exacto del ciclo:

```
Auth → Payment → Billing → Auth
```

---

## Esquema de la base de datos

```
servicios
├── id            UUID  PK
├── nombre        VARCHAR(100)  UNIQUE NOT NULL
├── descripcion   TEXT
├── activo        BOOLEAN DEFAULT TRUE
├── creado_en     TIMESTAMPTZ
└── actualizado_en TIMESTAMPTZ  (auto-actualizado por trigger)

dependencias
├── id            UUID  PK
├── origen        VARCHAR(100)  FK → servicios.nombre
├── destino       VARCHAR(100)  FK → servicios.nombre
├── descripcion   TEXT
└── creado_en     TIMESTAMPTZ

analisis
├── id                UUID  PK
├── tiene_ciclo       BOOLEAN
├── snapshot_grafo    JSONB
├── ciclos_detectados JSONB
└── ejecutado_en      TIMESTAMPTZ
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
| Monroy Rodríguez, Melvin José | MR24075 | Arquitectura general, API REST (Axum), coordinación |
| Escobar Arriaga, Josué Giovany | EA24012 | Persistencia PostgreSQL, sqlx, migraciones |
| Palma Rodriguez, Carlos Benito | PR24039 | Motor de grafos, integración petgraph |
| Polanco Vega, Bryan Moisés | PV21034 | Algoritmo DFS, detección de ciclos |
| Quinteros Rivas, Jonathan Steven | QR2400 | Testing, pruebas de integración, validación |

---

> **Asignatura:** Estructura de Datos 2 · **Docente:** Ing. William Zamora · **Grupo:** GT2  
> Universidad de El Salvador — Facultad Multidisciplinaria de Occidente
