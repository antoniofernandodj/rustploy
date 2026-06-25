#!/usr/bin/env bash
# Migra todos os IDs da base de dados para incluir prefixo de tipo:
#   project    → prj_<ulid>
#   service    → svc_<ulid>
#   deployment → dep_<ulid>
#   secret     → sec_<ulid>
#   git_provider → gp_<ulid>
#   tls_cert   → tls_<ulid>
set -euo pipefail

DB="/var/lib/rustploy/db/rustploy.db"
BACKUP="${DB}.bak_$(date +%Y%m%d_%H%M%S)"
SQL_FILE="$(mktemp /tmp/rustploy_migrate_XXXXXX.sql)"
trap 'rm -f "$SQL_FILE"' EXIT

echo "=== rustploy: migração de prefixos de IDs ==="
echo ""

# ── 1. Parar daemon ─────────────────────────────────────────────────────────
echo "[1/5] A parar rustployd..."
sudo systemctl stop rustployd
echo "      Parado."

# ── 2. Backup ────────────────────────────────────────────────────────────────
echo "[2/5] A fazer backup: $BACKUP"
sudo cp "$DB" "$BACKUP"
echo "      Backup criado."

# ── 3. SQL migration ─────────────────────────────────────────────────────────
echo "[3/5] A migrar IDs na base de dados..."

cat > "$SQL_FILE" <<'ENDSQL'
PRAGMA journal_mode=WAL;
BEGIN TRANSACTION;

-- ── Git providers ─────────────────────────────────────────────────────────
-- Actualizar provider_id dentro do JSON spec.source.Git (FK para git_provider)
UPDATE service
SET spec = json_set(
    spec,
    '$.source.Git.provider_id',
    'gp_' || json_extract(spec, '$.source.Git.provider_id')
)
WHERE json_extract(spec, '$.source.Git.provider_id') IS NOT NULL
  AND json_extract(spec, '$.source.Git.provider_id') NOT LIKE 'gp_%';

-- PK da tabela git_provider
UPDATE git_provider SET id = 'gp_' || id WHERE id NOT LIKE 'gp_%';

-- ── Projects ──────────────────────────────────────────────────────────────
-- project_id dentro do JSON spec (campo duplicado no blob)
UPDATE service
SET spec = json_set(spec, '$.project_id', 'prj_' || json_extract(spec, '$.project_id'))
WHERE json_extract(spec, '$.project_id') NOT LIKE 'prj_%';

-- FKs antes da PK
UPDATE service SET project_id = 'prj_' || project_id WHERE project_id NOT LIKE 'prj_%';
UPDATE secret  SET project_id = 'prj_' || project_id WHERE project_id NOT LIKE 'prj_%';

-- PK
UPDATE project SET id = 'prj_' || id WHERE id NOT LIKE 'prj_%';

-- ── Secrets ───────────────────────────────────────────────────────────────
UPDATE secret SET id = 'sec_' || id WHERE id NOT LIKE 'sec_%';

-- ── Services ──────────────────────────────────────────────────────────────
-- FKs antes da PK
UPDATE deployment    SET service_id = 'svc_' || service_id WHERE service_id NOT LIKE 'svc_%';
UPDATE webhook_token SET service_id = 'svc_' || service_id WHERE service_id NOT LIKE 'svc_%';

-- PK
UPDATE service SET id = 'svc_' || id WHERE id NOT LIKE 'svc_%';

-- ── Deployments ───────────────────────────────────────────────────────────
-- FK antes da PK
UPDATE build_log  SET deployment_id = 'dep_' || deployment_id WHERE deployment_id NOT LIKE 'dep_%';

-- PK
UPDATE deployment SET id = 'dep_' || id WHERE id NOT LIKE 'dep_%';

-- ── TLS certs ─────────────────────────────────────────────────────────────
UPDATE tls_cert SET id = 'tls_' || id WHERE id NOT LIKE 'tls_%';

COMMIT;
ENDSQL

sudo sqlite3 "$DB" < "$SQL_FILE"
echo "      Base de dados migrada."

# ── Verificação rápida ───────────────────────────────────────────────────────
echo ""
echo "      Verificação:"
sudo sqlite3 "$DB" "
SELECT '  projects:    ' || count(*) || ' (sample: ' || coalesce(min(id),'none') || ')' FROM project;
SELECT '  services:    ' || count(*) || ' (sample: ' || coalesce(min(id),'none') || ')' FROM service;
SELECT '  deployments: ' || count(*) || ' (sample: ' || coalesce(min(id),'none') || ')' FROM deployment;
SELECT '  git_providers:' || count(*) || ' (sample: ' || coalesce(min(id),'none') || ')' FROM git_provider;
"
echo ""

# ── 4. Rebuild e reinstall ───────────────────────────────────────────────────
echo "[4/5] A recompilar e reinstalar daemon..."
cd "$(dirname "$0")/.."
make install-daemon
echo "      Daemon reinstalado."

# ── 5. Iniciar daemon ────────────────────────────────────────────────────────
echo "[5/5] A iniciar rustployd..."
sudo systemctl start rustployd
sleep 1
STATUS=$(sudo systemctl is-active rustployd)
echo "      Estado: $STATUS"

echo ""
echo "=== Migração concluída! ==="
echo "Backup disponível em: $BACKUP"
