#!/bin/bash
# ============================================================
# demo_cycle.sh
# Script de demostración - ServiceMesh Analyzer
# Autor: Jonathan (QR24001)
# Descripción: Demuestra la detección de ciclos mediante
#              el endpoint REST /analyze
# ============================================================

BASE_URL="http://localhost:3000"

echo "============================================================"
echo "   ServiceMesh Analyzer - Demostración de Detección de Ciclos"
echo "============================================================"
echo ""

# ------------------------------------------------------------
# ESCENARIO 1: Registrar servicios
# ------------------------------------------------------------
echo "[1/5] Registrando microservicios..."
echo ""

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"name": "AuthService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"name": "PaymentService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/services" \
  -H "Content-Type: application/json" \
  -d '{"name": "BillingService"}' | python -m json.tool

echo ""
echo "[2/5] Servicios registrados correctamente."
echo ""

# ------------------------------------------------------------
# ESCENARIO 2: Crear dependencias SIN ciclo
# ------------------------------------------------------------
echo "[3/5] Creando dependencias sin ciclo..."
echo "      AuthService -> PaymentService -> BillingService"
echo ""

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"from": "AuthService", "to": "PaymentService"}' | python -m json.tool

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"from": "PaymentService", "to": "BillingService"}' | python -m json.tool

echo ""
echo "[4/5] Analizando grafo (esperado: SIN CICLOS)..."
echo ""

curl -s -X GET "$BASE_URL/analyze" | python -m json.tool

echo ""
echo "------------------------------------------------------------"
echo ""

# ------------------------------------------------------------
# ESCENARIO 3: Crear dependencia que genera ciclo
# ------------------------------------------------------------
echo "[5/5] Agregando dependencia circular..."
echo "      BillingService -> AuthService (genera ciclo!)"
echo ""

curl -s -X POST "$BASE_URL/deps" \
  -H "Content-Type: application/json" \
  -d '{"from": "BillingService", "to": "AuthService"}' | python -m json.tool

echo ""
echo "Analizando grafo (esperado: CICLO DETECTADO)..."
echo ""

curl -s -X GET "$BASE_URL/analyze" | python -m json.tool

echo ""
echo "============================================================"
echo "   Demostración completada."
echo "============================================================"