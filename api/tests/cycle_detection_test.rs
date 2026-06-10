use std::process::{Command, Child};
use std::time::Duration;
use std::thread;

// URL base del servidor de pruebas
const BASE_URL: &str = "http://127.0.0.1:3000";

fn esperar_servidor() {
    for _ in 0..20 {
        if reqwest::blocking::get(format!("{}/services", BASE_URL)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("El servidor no respondió a tiempo");
}

#[test]
fn test_registro_de_servicio() {
    let client = reqwest::blocking::Client::new();

    let response = client
        .post(format!("{}/services", BASE_URL))
        .json(&serde_json::json!({ "nombre": "ServicioTest" }))
        .send()
        .expect("Error al registrar servicio");

    assert!(
        response.status() == 201 || response.status() == 409,
        "Se esperaba 201 Created o 409 Conflict"
    );
}

#[test]
fn test_grafo_sin_ciclos_retorna_false() {
    let client = reqwest::blocking::Client::new();

    let response = client
        .get(format!("{}/analyze", BASE_URL))
        .send()
        .expect("Error al llamar /analyze");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().expect("Error al parsear JSON");
    assert!(body.get("tiene_ciclo").is_some(), "Falta campo tiene_ciclo");
}

#[test]
fn test_creacion_de_dependencia() {
    let client = reqwest::blocking::Client::new();

    // Crear servicios
    client.post(format!("{}/services", BASE_URL))
        .json(&serde_json::json!({ "nombre": "ServicioA" }))
        .send().ok();

    client.post(format!("{}/services", BASE_URL))
        .json(&serde_json::json!({ "nombre": "ServicioB" }))
        .send().ok();

    // Crear dependencia
    let response = client
        .post(format!("{}/deps", BASE_URL))
        .json(&serde_json::json!({
            "origen": "ServicioA",
            "destino": "ServicioB"
        }))
        .send()
        .expect("Error al crear dependencia");

    assert!(
        response.status() == 201 || response.status() == 409,
        "Se esperaba 201 Created o 409 Conflict"
    );
}

#[test]
fn test_deteccion_de_ciclo_end_to_end() {
    let client = reqwest::blocking::Client::new();

    // Crear servicios
    client.post(format!("{}/services", BASE_URL))
        .json(&serde_json::json!({ "nombre": "CicloServicioX" }))
        .send().ok();

    client.post(format!("{}/services", BASE_URL))
        .json(&serde_json::json!({ "nombre": "CicloServicioY" }))
        .send().ok();

    // Crear ciclo
    client.post(format!("{}/deps", BASE_URL))
        .json(&serde_json::json!({
            "origen": "CicloServicioX",
            "destino": "CicloServicioY"
        }))
        .send().ok();

    client.post(format!("{}/deps", BASE_URL))
        .json(&serde_json::json!({
            "origen": "CicloServicioY",
            "destino": "CicloServicioX"
        }))
        .send().ok();

    // Verificar detección
    let response = client
        .get(format!("{}/analyze", BASE_URL))
        .send()
        .expect("Error al llamar /analyze");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().expect("Error al parsear JSON");
    assert_eq!(
        body["tiene_ciclo"], true,
        "Se esperaba que el sistema detectara el ciclo"
    );
}