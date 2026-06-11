use reqwest::Client;
use serde_json::json;
use std::process::{Command, Child};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

/// Helper que levanta el servidor API en background para las pruebas.
/// Se encarga de hacer kill() automáticamente al final de la prueba.
struct ServerGuard {
    child: Child,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn start_server() -> ServerGuard {
    // Compila y ejecuta la aplicación (requiere base de datos ya arriba)
    let child = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("api")
        .spawn()
        .expect("No se pudo iniciar el servidor API para las pruebas");

    // Esperar a que el servidor compile e inicie el puerto 3000
    let client = Client::new();
    let mut up = false;
    for _ in 0..60 { // Esperar hasta 30 segundos
        if client.get("http://127.0.0.1:3000/services").send().await.is_ok() {
            up = true;
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    if !up {
        panic!("El servidor API no estuvo listo a tiempo o falló al iniciar.");
    }

    ServerGuard { child }
}

#[tokio::test]
async fn prueba_flujo_completo_con_ciclo() {
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    // Usamos un sufijo único para evitar conflictos con registros existentes en la BD persistente
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv_a = format!("srv_a_{}", suffix);
    let srv_b = format!("srv_b_{}", suffix);
    let srv_c = format!("srv_c_{}", suffix);

    // 1. Crear servicios en el API
    for srv in [&srv_a, &srv_b, &srv_c] {
        let resp = client.post(&format!("{}/services", base_url))
            .json(&json!({ "nombre": srv, "descripcion": format!("Test Service {}", srv) }))
            .send().await.expect("Error de red al conectar al API");
            
        assert!(resp.status().is_success(), "Fallo al crear servicio {}", srv);
    }

    // 2. Crear dependencias A -> B y B -> C (Aún no hay ciclo)
    let deps = vec![
        (&srv_a, &srv_b),
        (&srv_b, &srv_c),
    ];
    for (origen, destino) in deps {
        let resp = client.post(&format!("{}/deps", base_url))
            .json(&json!({ "origen": origen, "destino": destino }))
            .send().await.unwrap();
        assert!(resp.status().is_success(), "Fallo al crear dependencia {} -> {}", origen, destino);
    }

    // 3. Introducir ciclo C -> A
    let resp = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_c, "destino": &srv_a }))
        .send().await.unwrap();
    assert!(resp.status().is_success(), "Fallo al introducir dependencia circular");

    // 4. Llamar al endpoint de análisis y verificar que el GraphEngine detecta el ciclo
    let resp_analisis = client.get(&format!("{}/analyze", base_url))
        .send().await.unwrap();
        
    assert!(resp_analisis.status().is_success(), "Fallo en la ejecución del análisis");
    
    let body: serde_json::Value = resp_analisis.json().await.unwrap();

    // Validar el resultado principal del motor
    assert_eq!(body["tiene_ciclo"], true, "El GraphEngine debería haber detectado el ciclo que acabamos de introducir");
    assert!(body["alerta"].as_str().unwrap().contains("ALERTA CRÍTICA"), "El mensaje de alerta no indica ciclo");
}
#[tokio::test]
async fn prueba_grafo_sin_ciclos() {
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv_a = format!("sin_ciclo_a_{}", suffix);
    let srv_b = format!("sin_ciclo_b_{}", suffix);
    let srv_c = format!("sin_ciclo_c_{}", suffix);

    for srv in [&srv_a, &srv_b, &srv_c] {
        client.post(&format!("{}/services", base_url))
            .json(&json!({ "nombre": srv }))
            .send().await.unwrap();
    }

    // A -> B -> C (sin ciclo)
    client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
        .send().await.unwrap();

    client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_b, "destino": &srv_c }))
        .send().await.unwrap();

    let resp = client.get(&format!("{}/analyze", base_url))
        .send().await.unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["tiene_ciclo"], false, "No debería detectar ciclo en topología válida");
}

#[tokio::test]
async fn prueba_servicio_duplicado_es_rechazado() {
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv = format!("duplicado_{}", suffix);

    // Primer registro debe funcionar
    let resp1 = client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();
    assert!(resp1.status().is_success(), "El primer registro debería ser exitoso");

    // Segundo registro del mismo servicio debe ser rechazado
    let resp2 = client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();
    assert!(
        resp2.status() == 409 || resp2.status() == 400,
        "El servicio duplicado debería ser rechazado con 409 o 400"
    );
}

#[tokio::test]
async fn prueba_dependencia_duplicada_es_rechazada() {
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv_a = format!("dep_dup_a_{}", suffix);
    let srv_b = format!("dep_dup_b_{}", suffix);

    for srv in [&srv_a, &srv_b] {
        client.post(&format!("{}/services", base_url))
            .json(&json!({ "nombre": srv }))
            .send().await.unwrap();
    }

    // Primera dependencia debe funcionar
    let resp1 = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
        .send().await.unwrap();
    assert!(resp1.status().is_success(), "La primera dependencia debería crearse");

    // Segunda dependencia igual debe ser rechazada
    let resp2 = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
        .send().await.unwrap();
    assert!(
        resp2.status() == 409 || resp2.status() == 400,
        "La dependencia duplicada debería ser rechazada"
    );
}

#[tokio::test]
async fn prueba_self_loop_es_rechazado() {
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv = format!("self_loop_{}", suffix);

    client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();

    // Un servicio que depende de sí mismo debe ser rechazado
    let resp = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv, "destino": &srv }))
        .send().await.unwrap();

    assert!(
        resp.status() == 400 || resp.status() == 422,
        "Un self-loop debería ser rechazado por la API"
    );
}