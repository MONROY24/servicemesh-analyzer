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
