CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS servicios (
    id             UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre         VARCHAR(100) NOT NULL,
    descripcion    TEXT,
    activo         BOOLEAN      NOT NULL DEFAULT TRUE,
    creado_en      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_servicios_nombre UNIQUE (nombre),
    CONSTRAINT chk_servicios_nombre_no_vacio CHECK (TRIM(nombre) <> '')
);

CREATE TABLE IF NOT EXISTS dependencias (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    origen      VARCHAR(100) NOT NULL,
    destino     VARCHAR(100) NOT NULL,
    descripcion TEXT,
    creado_en   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_no_self_loop CHECK (origen <> destino),
    CONSTRAINT uq_dependencias_par UNIQUE (origen, destino),
    CONSTRAINT fk_dep_origen FOREIGN KEY (origen) REFERENCES servicios(nombre) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT fk_dep_destino FOREIGN KEY (destino) REFERENCES servicios(nombre) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS analisis (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tiene_ciclo       BOOLEAN     NOT NULL,
    snapshot_grafo    JSONB,
    ciclos_detectados JSONB,
    ejecutado_en      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_servicios_nombre ON servicios(nombre);
CREATE INDEX IF NOT EXISTS idx_dependencias_origen ON dependencias(origen);
CREATE INDEX IF NOT EXISTS idx_dependencias_destino ON dependencias(destino);
CREATE INDEX IF NOT EXISTS idx_analisis_ejecutado_en ON analisis(ejecutado_en DESC);
CREATE INDEX IF NOT EXISTS idx_analisis_tiene_ciclo ON analisis(tiene_ciclo) WHERE tiene_ciclo = TRUE;

CREATE OR REPLACE FUNCTION fn_actualizar_timestamp()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.actualizado_en = NOW();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_servicios_actualizado_en ON servicios;

CREATE TRIGGER trg_servicios_actualizado_en
    BEFORE UPDATE ON servicios
    FOR EACH ROW EXECUTE FUNCTION fn_actualizar_timestamp();

CREATE OR REPLACE VIEW vista_grafo AS
SELECT
    d.id            AS dep_id,
    d.origen,
    s1.descripcion  AS desc_origen,
    d.destino,
    s2.descripcion  AS desc_destino,
    d.descripcion   AS desc_dependencia,
    d.creado_en
FROM dependencias d
JOIN servicios s1 ON s1.nombre = d.origen
JOIN servicios s2 ON s2.nombre = d.destino
ORDER BY d.origen, d.destino;

CREATE OR REPLACE VIEW vista_ultimo_analisis AS
SELECT *
FROM analisis
ORDER BY ejecutado_en DESC
LIMIT 1;

CREATE OR REPLACE VIEW vista_servicios_raiz AS
SELECT s.nombre, s.descripcion
FROM servicios s
WHERE s.activo = TRUE
  AND s.nombre NOT IN (SELECT destino FROM dependencias);

CREATE OR REPLACE VIEW vista_servicios_hoja AS
SELECT s.nombre, s.descripcion
FROM servicios s
WHERE s.activo = TRUE
  AND s.nombre NOT IN (SELECT origen FROM dependencias);

INSERT INTO servicios (nombre, descripcion) VALUES
    ('AuthService',         'Autenticación y autorización de usuarios'),
    ('PaymentService',      'Procesamiento de pagos y transacciones'),
    ('BillingService',      'Facturación y generación de recibos'),
    ('NotificationService', 'Envío de notificaciones email/SMS'),
    ('LogService',          'Agregación centralizada de logs'),
    ('InventoryService',    'Gestión de inventario de productos'),
    ('OrderService',        'Gestión del ciclo de vida de órdenes')
ON CONFLICT (nombre) DO NOTHING;

INSERT INTO dependencias (origen, destino, descripcion) VALUES
    ('OrderService',        'AuthService',      'Valida token antes de procesar orden'),
    ('OrderService',        'InventoryService', 'Verifica stock disponible'),
    ('OrderService',        'PaymentService',   'Cobra el pago de la orden'),
    ('PaymentService',      'AuthService',      'Verifica identidad del pagador'),
    ('PaymentService',      'LogService',       'Registra eventos de pago'),
    ('BillingService',      'PaymentService',   'Consulta estado del pago'),
    ('BillingService',      'LogService',       'Registra eventos de facturación'),
    ('AuthService',         'LogService',       'Registra intentos de autenticación'),
    ('NotificationService', 'LogService',       'Registra notificaciones enviadas')
ON CONFLICT DO NOTHING;

DO $$
DECLARE
    n_servicios    INT;
    n_dependencias INT;
BEGIN
    SELECT COUNT(*) INTO n_servicios    FROM servicios;
    SELECT COUNT(*) INTO n_dependencias FROM dependencias;
    RAISE NOTICE '✓ Schema creado correctamente.';
    RAISE NOTICE '  Servicios insertados   : %', n_servicios;
    RAISE NOTICE '  Dependencias insertadas: %', n_dependencias;
END;
$$;
