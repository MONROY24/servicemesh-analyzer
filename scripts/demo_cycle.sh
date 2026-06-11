#!/bin/bash
# ============================================================
# demo_cycle.sh
# Script de demostración - ServiceMesh Analyzer
# Autor: Jonathan (QR24001)
# ============================================================

BASE_URL="http://localhost:3000"

echo "============================================================"
echo "   ServiceMesh Analyzer - Demostración de Detección de Ciclos"
echo "============================================================"
echo ""

echo "[1/5] Registrando microservicios..."
echo ""

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"nombre": "AuthService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"nombre": "PaymentService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"nombre": "BillingService"}' | python -m json.tool

echo ""
echo "[2/5] Servicios registrados correctamente."
echo ""

echo "[3/5] Creando dependencias sin ciclo..."
echo "      AuthService -> PaymentService -> BillingService"
echo ""

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"origen": "AuthService", "destino": "PaymentService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"origen": "PaymentService", "destino": "BillingService"}' | python -m json.tool

echo ""
echo "[4/5] Analizando grafo (esperado: SIN CICLOS)..."
echo ""

curl -s -X GET "$BASE_URL/analyze" | python -m json.tool

echo ""
echo "------------------------------------------------------------"
echo ""

echo "[5/5] Agregando dependencia circular..."
echo "      BillingService -> AuthService (genera ciclo!)"
echo ""

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"origen": "BillingService", "destino": "AuthService"}' | python -m json.tool

echo ""
echo "Analizando grafo (esperado: CICLO DETECTADO)..."
echo ""

curl -s -X GET "$BASE_URL/analyze" | python -m json.tool

echo ""
echo "============================================================"
echo "   Demostración completada."
echo "============================================================"