$baseUrl = "http://localhost:3000"

Write-Host "============================================================"
Write-Host "   ServiceMesh Analyzer - Demostración de Detección de Ciclos"
Write-Host "============================================================"
Write-Host ""

Write-Host "[1/5] Registrando microservicios..."
Write-Host ""

curl.exe -s -X POST "$baseUrl/services" -H "Content-Type: application/json" -d '{"nombre": "AuthService"}'
curl.exe -s -X POST "$baseUrl/services" -H "Content-Type: application/json" -d '{"nombre": "PaymentService"}'
curl.exe -s -X POST "$baseUrl/services" -H "Content-Type: application/json" -d '{"nombre": "BillingService"}'

Write-Host ""
Write-Host "[2/5] Servicios registrados correctamente."
Write-Host ""

Write-Host "[3/5] Creando dependencias sin ciclo..."
Write-Host "      AuthService -> PaymentService -> BillingService"
Write-Host ""

curl.exe -s -X POST "$baseUrl/deps" -H "Content-Type: application/json" -d '{"origen": "AuthService", "destino": "PaymentService"}'
curl.exe -s -X POST "$baseUrl/deps" -H "Content-Type: application/json" -d '{"origen": "PaymentService", "destino": "BillingService"}'

Write-Host ""
Write-Host "[4/5] Analizando grafo (esperado: SIN CICLOS)..."
Write-Host ""

curl.exe -s -X GET "$baseUrl/analyze"

Write-Host ""
Write-Host "------------------------------------------------------------"
Write-Host ""

Write-Host "[5/5] Agregando dependencia circular..."
Write-Host "      BillingService -> AuthService (genera ciclo!)"
Write-Host ""

curl.exe -s -X POST "$baseUrl/deps" -H "Content-Type: application/json" -d '{"origen": "BillingService", "destino": "AuthService"}'

Write-Host ""
Write-Host "Analizando grafo (esperado: CICLO DETECTADO)..."
Write-Host ""

curl.exe -s -X GET "$baseUrl/analyze"

Write-Host ""
Write-Host "============================================================"
Write-Host "   Demostración completada."
Write-Host "============================================================"
