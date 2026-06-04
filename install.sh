#!/bin/sh
# Instala rustployd e rustploy a partir do GitHub Releases.
# Uso: curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh
# Ou:  sudo RUSTPLOY_VERSION=v0.2.0 sh install.sh
set -e

REPO="antoniofernandodj/rustploy"
INSTALL_DIR="/usr/local/bin"
SERVICE_FILE="/etc/systemd/system/rustployd.service"
CONFIG_DIR="/etc/rustploy"
DATA_DIR="/var/lib/rustploy"

# ── Verificações ──────────────────────────────────────────────────────────────

if [ "$(id -u)" -ne 0 ]; then
    echo "erro: execute este script como root (sudo sh install.sh)" >&2
    exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "erro: Docker não encontrado. Instale o Docker antes de continuar." >&2
    exit 1
fi

if ! command -v systemctl >/dev/null 2>&1; then
    echo "erro: systemd não encontrado." >&2
    exit 1
fi

# ── Detectar arquitetura ──────────────────────────────────────────────────────

ARCH=$(uname -m)
case "$ARCH" in
    x86_64)          ARCH_SUFFIX="x86_64" ;;
    aarch64|arm64)   ARCH_SUFFIX="aarch64" ;;
    *)
        echo "erro: arquitetura não suportada: $ARCH" >&2
        exit 1
        ;;
esac

# ── Resolver versão ───────────────────────────────────────────────────────────

VERSION="${RUSTPLOY_VERSION:-}"
if [ -z "$VERSION" ]; then
    echo "Buscando versão mais recente..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
fi

if [ -z "$VERSION" ]; then
    echo "erro: não foi possível determinar a versão. Defina RUSTPLOY_VERSION=vX.Y.Z" >&2
    exit 1
fi

echo "Instalando rustploy $VERSION ($ARCH_SUFFIX)..."

# ── Download ──────────────────────────────────────────────────────────────────

TARBALL="rustploy-${VERSION}-${ARCH_SUFFIX}.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION/$TARBALL"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Baixando $URL..."
curl -fsSL "$URL" -o "$TMP/$TARBALL"
tar -xzf "$TMP/$TARBALL" -C "$TMP"

# ── Instalar binários ─────────────────────────────────────────────────────────

install -m 755 "$TMP/rustployd" "$INSTALL_DIR/rustployd"
install -m 755 "$TMP/rustploy"  "$INSTALL_DIR/rustploy"
echo "Binários instalados em $INSTALL_DIR/"

# ── Criar usuário de sistema ──────────────────────────────────────────────────

if ! getent group docker >/dev/null 2>&1; then
    groupadd --system docker || true
fi

if ! id -u rustploy >/dev/null 2>&1; then
    useradd \
        --system \
        --no-create-home \
        --shell /usr/sbin/nologin \
        --groups docker \
        --comment "Rustploy daemon" \
        rustploy
    echo "Usuário 'rustploy' criado."
fi

# ── Criar diretórios ──────────────────────────────────────────────────────────

mkdir -p "$CONFIG_DIR" "$DATA_DIR"
chown rustploy:rustploy "$CONFIG_DIR" "$DATA_DIR"
chmod 750 "$CONFIG_DIR"

# ── Instalar service do systemd ───────────────────────────────────────────────

if [ -f "$TMP/rustployd.service" ]; then
    # O tarball inclui o .service — usa ele diretamente
    sed "s|ExecStart=.*|ExecStart=$INSTALL_DIR/rustployd|" \
        "$TMP/rustployd.service" > "$SERVICE_FILE"
else
    # Fallback: gera o service inline
    cat > "$SERVICE_FILE" << EOF
[Unit]
Description=Rustploy PaaS Daemon
After=network.target docker.service
Wants=docker.service

[Service]
Type=simple
User=rustploy
Group=rustploy
SupplementaryGroups=docker
ExecStart=$INSTALL_DIR/rustployd
Restart=on-failure
RestartSec=5
TimeoutStopSec=30

RuntimeDirectory=rustploy
RuntimeDirectoryMode=0755
StateDirectory=rustploy
ConfigurationDirectory=rustploy

Environment=RUSTPLOY_SOCKET_PATH=/run/rustploy/rustploy.sock
Environment=RUSTPLOY_DB_PATH=/var/lib/rustploy/db
Environment=RUST_LOG=info

NoNewPrivileges=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOF
fi

chmod 644 "$SERVICE_FILE"
echo "Service instalado em $SERVICE_FILE"

# ── Ativar e iniciar ──────────────────────────────────────────────────────────

systemctl daemon-reload
systemctl enable rustployd
systemctl restart rustployd

echo ""
echo "rustploy $VERSION instalado com sucesso!"
echo "  Daemon:  systemctl status rustployd"
echo "  Cliente: rustploy"
echo "  Config:  $CONFIG_DIR/config.toml"
echo "  Logs:    journalctl -u rustployd -f"
