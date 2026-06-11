use reqwest::Client;
use serde_json::json;
use std::process::{Command, Child};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use std::sync::Mutex;

// Mutex global para serializar los tests de integración y evitar colisiones de puerto 3000 y base de datos
static TEST_MUTEX: Mutex<()> = Mutex::new(());

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
    let child = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("api")
        .spawn()
        .expect("No se pudo iniciar el servidor API para las pruebas");

    let client = Client::new();
    let mut up = false;
    for _ in 0..60 {
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

async fn cleanup_services(client: &Client, base_url: &str, services: &[&str]) {
    for srv in services {
        let _ = client.delete(&format!("{}/services/{}", base_url, srv)).send().await;
    }
}

#[tokio::test]
async fn prueba_flujo_completo_con_ciclo() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv_a = format!("srv_a_{}", suffix);
    let srv_b = format!("srv_b_{}", suffix);
    let srv_c = format!("srv_c_{}", suffix);

    for srv in [&srv_a, &srv_b, &srv_c] {
        let resp = client.post(&format!("{}/services", base_url))
            .json(&json!({ "nombre": srv, "descripcion": format!("Test Service {}", srv) }))
            .send().await.expect("Error de red al conectar al API");
        assert!(resp.status().is_success(), "Fallo al crear servicio {}", srv);
    }

    let deps = vec![(&srv_a, &srv_b), (&srv_b, &srv_c)];
    for (origen, destino) in deps {
        let resp = client.post(&format!("{}/deps", base_url))
            .json(&json!({ "origen": origen, "destino": destino }))
            .send().await.unwrap();
        assert!(resp.status().is_success(), "Fallo al crear dependencia {} -> {}", origen, destino);
    }

    let resp = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_c, "destino": &srv_a }))
        .send().await.unwrap();
    assert!(resp.status().is_success(), "Fallo al introducir dependencia circular");

    let resp_analisis = client.get(&format!("{}/analyze", base_url))
        .send().await.unwrap();
    assert!(resp_analisis.status().is_success(), "Fallo en la ejecución del análisis");

    let body: serde_json::Value = resp_analisis.json().await.unwrap();
    assert_eq!(body["tiene_ciclo"], true, "El GraphEngine debería haber detectado el ciclo");
    assert!(body["alerta"].as_str().unwrap().contains("ALERTA CRÍTICA"), "El mensaje de alerta no indica ciclo");

    cleanup_services(&client, base_url, &[&srv_a, &srv_b, &srv_c]).await;
}

#[tokio::test]
async fn prueba_grafo_sin_ciclos() {
    let _lock = TEST_MUTEX.lock().unwrap();
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

    client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
        .send().await.unwrap();

    client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_b, "destino": &srv_c }))
        .send().await.unwrap();

    let resp = {
        let mut resultado = None;
        for _ in 0..3 {
            match client.get(&format!("{}/analyze", base_url)).send().await {
                Ok(r) => { resultado = Some(r); break; }
                Err(_) => { sleep(Duration::from_millis(500)).await; }
            }
        }
        resultado.expect("No se pudo conectar al endpoint /analyze después de 3 intentos")
    };

    let body: serde_json::Value = resp.json().await.unwrap();

    let empty = vec![];
    let ciclos = body["ciclos_detectados"].as_array().unwrap_or(&empty);
    let servicios_en_ciclo: Vec<String> = ciclos.iter()
        .flat_map(|c| c.as_array().unwrap_or(&empty).iter())
        .filter_map(|s| s.as_str())
        .map(|s| s.to_string())
        .collect();

    assert!(
        !servicios_en_ciclo.contains(&srv_a) &&
        !servicios_en_ciclo.contains(&srv_b) &&
        !servicios_en_ciclo.contains(&srv_c),
        "Los servicios de prueba no deberían estar en ningún ciclo"
    );

    cleanup_services(&client, base_url, &[&srv_a, &srv_b, &srv_c]).await;
}

#[tokio::test]
async fn prueba_servicio_duplicado_es_rechazado() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv = format!("duplicado_{}", suffix);

    let resp1 = client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();
    assert!(resp1.status().is_success(), "El primer registro debería ser exitoso");

    let resp2 = client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();
    assert!(
        resp2.status() == 409 || resp2.status() == 400,
        "El servicio duplicado debería ser rechazado con 409 o 400"
    );

    cleanup_services(&client, base_url, &[&srv]).await;
}

#[tokio::test]
async fn prueba_dependencia_duplicada_es_rechazada() {
    let _lock = TEST_MUTEX.lock().unwrap();
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

    let resp1 = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
        .send().await.unwrap();
    assert!(resp1.status().is_success(), "La primera dependencia debería crearse");

    let resp2 = {
        let mut resultado = None;
        for _ in 0..3 {
            match client.post(&format!("{}/deps", base_url))
                .json(&json!({ "origen": &srv_a, "destino": &srv_b }))
                .send().await {
                Ok(r) => { resultado = Some(r); break; }
                Err(_) => { sleep(Duration::from_millis(500)).await; }
            }
        }
        resultado.expect("No se pudo conectar al endpoint /deps después de 3 intentos")
    };

    assert!(
        resp2.status() == 409 || resp2.status() == 400,
        "La dependencia duplicada debería ser rechazada"
    );

    cleanup_services(&client, base_url, &[&srv_a, &srv_b]).await;
}

#[tokio::test]
async fn prueba_self_loop_es_rechazado() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let _guard = start_server().await;
    let client = Client::new();
    let base_url = "http://127.0.0.1:3000";

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let srv = format!("self_loop_{}", suffix);

    client.post(&format!("{}/services", base_url))
        .json(&json!({ "nombre": srv }))
        .send().await.unwrap();

    let resp = client.post(&format!("{}/deps", base_url))
        .json(&json!({ "origen": &srv, "destino": &srv }))
        .send().await.unwrap();

    assert!(
        resp.status() == 400 || resp.status() == 422,
        "Un self-loop debería ser rechazado por la API"
    );

    cleanup_services(&client, base_url, &[&srv]).await;
}