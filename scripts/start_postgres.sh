#!/bin/bash
set -e

# Obtener el directorio donde se encuentra este script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR="$(dirname "$DIR")"

ENV_FILE="$ROOT_DIR/.env.example"

# Se intenta extraer la URL y limpiarla de retornos de carro
DB_URL=$(grep -i "DATABASE_URL=" "$ENV_FILE" 2>/dev/null | tr -d '\r\n\0' | cut -d '=' -f 2-)

# Parsear la URL de conexión de postgresql
if [[ $DB_URL =~ postgres://([^:]+):([^@]+)@([^:]+):([0-9]+)/([^[:space:]]+) ]]; then
    DB_USER="${BASH_REMATCH[1]}"
    DB_PASS="${BASH_REMATCH[2]}"
    DB_HOST="${BASH_REMATCH[3]}"
    DB_PORT="${BASH_REMATCH[4]}"
    DB_NAME="${BASH_REMATCH[5]}"
else
    # Valores por defecto que corresponden a .env.example
    DB_USER="postgres"
    DB_PASS="TU_PASSWORD"
    DB_PORT="5432"
    DB_NAME="servicemesh_analyzer"
fi

echo "Levantando contenedor Docker de PostgreSQL (Puerto: $DB_PORT, BD: $DB_NAME)..."

docker stop servicemesh-postgres 2>/dev/null || true
docker rm servicemesh-postgres 2>/dev/null || true

docker run --name servicemesh-postgres \
  -e POSTGRES_USER="$DB_USER" \
  -e POSTGRES_PASSWORD="$DB_PASS" \
  -e POSTGRES_DB="$DB_NAME" \
  -p "$DB_PORT":5432 \
  -d postgres:15-alpine

echo "Esperando 5 segundos a que la base de datos inicialice..."
sleep 5

echo "Ejecutando migrations/servicemesh_db.sql..."
docker exec -i servicemesh-postgres psql -U "$DB_USER" -d "$DB_NAME" < "$ROOT_DIR/migrations/servicemesh_db.sql"

echo "¡Contenedor PostgreSQL levantado y migraciones aplicadas correctamente!"
