#!/bin/sh
# Instalador do Rustploy (Debian, Ubuntu e derivados), a partir da última release do GitHub.
#
#   Daemon, na VPS:    curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh
#   App desktop:       curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh -s -- gui
#
# Opções (depois de `sh -s --`, ou direto: `sudo sh install.sh gui --version v0.1.1`):
#   daemon | gui        o que instalar (padrão: daemon)
#   --remote            (daemon) a API aceita conexões de fora da VPS (bind 0.0.0.0, com token).
#                       Sem isso, ela escuta só em 127.0.0.1 (use um túnel SSH ou rode de novo com --remote).
#   --engine docker|podman  (daemon) o motor de containers (padrão: docker). Em ambos o instalador instala
#                       o motor se faltar: docker = Docker Engine pelo repositório oficial da Docker;
#                       podman = Podman + compose, podman.socket, acesso do usuário rustploy e config.toml.
#   --version vX.Y.Z    uma release específica (padrão: a última)
#   --dry-run           só mostra o que baixaria e instalaria
#   -h | --help         esta ajuda
#
# Variáveis: RUSTPLOY_VERSION (igual a --version), RUSTPLOY_ENGINE (igual a --engine), RUSTPLOY_DRY_RUN=1,
#            RUSTPLOY_REPO (padrão antoniofernandodj/rustploy), RUSTPLOY_API_URL (só para testes).
#
# Windows: veja install.ps1 (irm .../install.ps1 | iex).
set -eu

REPO="${RUSTPLOY_REPO:-antoniofernandodj/rustploy}"
API="${RUSTPLOY_API_URL:-https://api.github.com/repos/$REPO/releases}"
DOCS="https://rustploy-docs.antoniofernandodj.workers.dev/instalacao"
WHAT="daemon"
VERSION="${RUSTPLOY_VERSION:-}"
DRY="${RUSTPLOY_DRY_RUN:-0}"
REMOTE=0
ENGINE="${RUSTPLOY_ENGINE:-docker}"
PODMAN_SOCKET="/run/podman/podman.sock"
CONFIG="/etc/rustploy/config.toml"

say() { printf '==> %s\n' "$*"; }
warn() { printf 'aviso: %s\n' "$*" >&2; }
die() { printf 'erro: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'HELP'
Instalador do Rustploy (Debian, Ubuntu e derivados), a partir da última release do GitHub.

Uso: sudo sh install.sh [daemon|gui] [--remote] [--engine docker|podman] [--version vX.Y.Z] [--dry-run]

  daemon            instala o rustployd na VPS (padrão)
  gui               instala o app desktop (amd64)
  --remote          (daemon) a API aceita conexões de fora da VPS (bind 0.0.0.0, com token);
                    sem isso, escuta só em 127.0.0.1
  --engine docker   (daemon) motor de containers = Docker (padrão); instala o Docker Engine se faltar
  --engine podman   (daemon) motor de containers = Podman; instala o Podman e configura tudo
  --version vX.Y.Z  uma release específica (padrão: a última)
  --dry-run         só mostra o que baixaria e instalaria
HELP
}

while [ $# -gt 0 ]; do
    case "$1" in
        daemon|gui) WHAT="$1" ;;
        --version) shift; [ $# -gt 0 ] || die "--version precisa de um valor (ex.: v0.1.1)"; VERSION="$1" ;;
        --version=*) VERSION="${1#--version=}" ;;
        --dry-run) DRY=1 ;;
        --remote) REMOTE=1 ;;
        --engine) shift; [ $# -gt 0 ] || die "--engine precisa de um valor (docker ou podman)"; ENGINE="$1" ;;
        --engine=*) ENGINE="${1#--engine=}" ;;
        -h|--help) usage; exit 0 ;;
        *) die "argumento desconhecido: $1 (use --help)" ;;
    esac
    shift
done

case "$ENGINE" in
    docker) ;;
    podman) [ "$WHAT" = "daemon" ] || die "--engine podman vale só para o daemon" ;;
    *) die "--engine inválido: $ENGINE (use docker ou podman)" ;;
esac

# ── Verificações ──────────────────────────────────────────────────────────────

command -v curl >/dev/null 2>&1 || die "curl não encontrado. Instale com: apt-get install -y curl"
command -v apt-get >/dev/null 2>&1 || die "este instalador é para Debian/Ubuntu (apt). Em outros sistemas, veja $DOCS/instalar-daemon/"
command -v dpkg >/dev/null 2>&1 || die "dpkg não encontrado"
# Subshell: o /etc/os-release define VERSION, ID etc. e não pode sobrescrever as variáveis daqui.
OS_IDS="$( [ -r /etc/os-release ] && . /etc/os-release; echo "${ID:-} ${ID_LIKE:-}" )"
case "$OS_IDS" in
    *debian*|*ubuntu*) ;;
    *) warn "sistema '${OS_IDS% *}' não é Debian/Ubuntu; seguindo mesmo assim (apt está presente)" ;;
esac

if [ "$DRY" != "1" ] && [ "$(id -u)" -ne 0 ]; then
    die "rode como root: curl -fsSL <url>/install.sh | sudo sh   (ou: sudo sh install.sh)"
fi

# O .deb do daemon exige glibc >= 2.34 e o do app >= 2.35 (Debian 12+, Ubuntu 22.04+ nos dois casos).
# Melhor avisar aqui do que deixar o apt falhar.
MIN_GLIBC="2.34"
[ "$WHAT" = "gui" ] && MIN_GLIBC="2.35"
GLIBC="$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')"
if [ -z "$GLIBC" ]; then
    warn "não consegui descobrir a versão da glibc (sistema sem glibc, como o Alpine, não é suportado)"
elif [ "$(printf '%s\n%s\n' "$MIN_GLIBC" "$GLIBC" | sort -V | head -n 1)" != "$MIN_GLIBC" ]; then
    die "glibc $GLIBC é antiga demais: o pacote exige glibc >= $MIN_GLIBC (Debian 12+, Ubuntu 22.04+).
       Veja os sistemas compatíveis em $DOCS/requisitos/"
fi

ARCH="$(dpkg --print-architecture)"
case "$WHAT:$ARCH" in
    daemon:amd64|daemon:arm64|gui:amd64) ;;
    gui:*) die "o app desktop só tem pacote para amd64 por enquanto (esta máquina é $ARCH)" ;;
    *) die "arquitetura sem pacote: $ARCH (suportadas: amd64 e arm64)" ;;
esac

case "$WHAT" in
    daemon) PREFIX="rustployd_" ;;
    gui)    PREFIX="rustploy-gui_" ;;
esac

# ── Resolver a release ────────────────────────────────────────────────────────

if [ -n "$VERSION" ]; then
    URL="$API/tags/$VERSION"
else
    URL="$API/latest"
fi

say "Procurando a release ${VERSION:-mais recente} de $REPO..."
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM

HTTP="$(curl -sSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: rustploy-installer' \
    -w '%{http_code}' -o "$TMP/release.json" "$URL" || true)"
case "$HTTP" in
    200) ;;
    404) if [ -n "$VERSION" ]; then
           die "a release $VERSION não existe em github.com/$REPO. Veja as disponíveis em https://github.com/$REPO/releases"
       fi
       die "nenhuma release publicada em github.com/$REPO.
       Enquanto não houver, instale pelo crates.io: cargo install $( [ "$WHAT" = gui ] && echo rustploy-gui || echo rustploy )
       (mais detalhes em $DOCS/instalar-$( [ "$WHAT" = gui ] && echo clientes || echo daemon )/)" ;;
    403|429) die "o GitHub limitou as consultas à API (HTTP $HTTP). Espere alguns minutos ou defina RUSTPLOY_VERSION=vX.Y.Z" ;;
    *) die "não consegui consultar $URL (HTTP ${HTTP:-sem resposta})" ;;
esac

# A API devolve o JSON numa linha só (minificado): por isso grep -o, e não sed por linha.
TAG="$(grep -o '"tag_name": *"[^"]*"' "$TMP/release.json" | head -n 1 | sed 's/.*: *"\(.*\)"/\1/')"
[ -n "$TAG" ] || die "resposta inesperada do GitHub (sem tag_name)"

assets() { grep -o '"browser_download_url": *"[^"]*"' "$TMP/release.json" | sed 's/.*: *"\(.*\)"/\1/'; }
DEB_URL="$(assets | grep "/${PREFIX}[^/]*_${ARCH}\.deb\$" | head -n 1 || true)"
SUMS_URL="$(assets | grep '/checksums\.sha256$' | head -n 1 || true)"
[ -n "$DEB_URL" ] || die "a release $TAG não tem pacote ${PREFIX}*_${ARCH}.deb"
DEB="${DEB_URL##*/}"

say "Release $TAG: $DEB"
if [ "$DRY" = "1" ]; then
    say "--dry-run: baixaria $DEB_URL"
    [ -n "$SUMS_URL" ] && say "--dry-run: conferiria o SHA-256 em $SUMS_URL"
    [ "$WHAT" = "daemon" ] && [ "$ENGINE" = "docker" ] && ! command -v docker >/dev/null 2>&1 && say "--dry-run: instalaria o Docker Engine pelo repositório oficial (download.docker.com)"
    [ "$ENGINE" = "podman" ] && say "--dry-run: instalaria o Podman (+ provider do compose), ligaria o podman.socket e apontaria o config.toml para $PODMAN_SOCKET"
    say "--dry-run: e instalaria com: apt-get install -y ./$DEB"
    exit 0
fi

# ── Baixar e conferir ─────────────────────────────────────────────────────────

say "Baixando $DEB..."
curl -fSL --progress-bar -o "$TMP/$DEB" "$DEB_URL" || die "falha ao baixar $DEB_URL"

if [ -n "$SUMS_URL" ]; then
    curl -fsSL -o "$TMP/checksums.sha256" "$SUMS_URL" || die "falha ao baixar $SUMS_URL"
    WANT="$(grep " \**${DEB}\$" "$TMP/checksums.sha256" | head -n 1 | cut -d ' ' -f 1 || true)"
    [ -n "$WANT" ] || die "o checksums.sha256 da release não lista $DEB"
    GOT="$(sha256sum "$TMP/$DEB" | cut -d ' ' -f 1)"
    [ "$WANT" = "$GOT" ] || die "SHA-256 de $DEB não confere (esperado $WANT, obtido $GOT). Não instalei nada."
    say "SHA-256 conferido."
else
    warn "a release não tem checksums.sha256; não consegui conferir o SHA-256"
fi

# ── Docker (padrão: --engine docker) ──────────────────────────────────────────

# Instala o Docker Engine pelo repositório apt oficial da Docker, seguindo
# https://docs.docker.com/engine/install/ubuntu/ (e a página equivalente do Debian).
# Se o `docker` já existe, não mexe em nada.
install_docker() {
    if command -v docker >/dev/null 2>&1; then
        say "Docker já instalado ($(docker --version 2>/dev/null | head -n 1)); mantido como está."
        return 0
    fi
    say "Instalando o Docker Engine (repositório oficial da Docker)..."

    # Subshell: o /etc/os-release define VERSION, ID etc. e não pode sobrescrever as variáveis daqui.
    DIST_INFO="$( . /etc/os-release; echo "${ID:-} ${ID_LIKE:-}|${UBUNTU_CODENAME:-}|${VERSION_CODENAME:-}" )"
    DIST_IDS="${DIST_INFO%%|*}"
    REST="${DIST_INFO#*|}"
    UB_CODENAME="${REST%%|*}"
    DEB_CODENAME="${REST#*|}"
    case " $DIST_IDS " in
        *" ubuntu "*) DOCKER_DIST="ubuntu"; CODENAME="${UB_CODENAME:-$DEB_CODENAME}" ;;
        *" debian "*) DOCKER_DIST="debian"; CODENAME="$DEB_CODENAME" ;;
        *) DOCKER_DIST="" ;;
    esac
    if [ -z "$DOCKER_DIST" ] || [ -z "$CODENAME" ]; then
        warn "não reconheci a distribuição ($DIST_IDS); instalando o docker.io do repositório da própria distribuição."
        DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io || die "não consegui instalar o Docker"
        return 0
    fi

    # 1. Pacotes que conflitam com o docker-ce (só os que estão instalados).
    CONFLICTS="$(dpkg --get-selections docker.io docker-compose docker-compose-v2 docker-doc docker-buildx \
        podman-docker containerd runc 2>/dev/null | awk '$2 == "install" {print $1}' | tr '\n' ' ')"
    # shellcheck disable=SC2086
    [ -n "$CONFLICTS" ] && DEBIAN_FRONTEND=noninteractive apt-get remove -y $CONFLICTS

    # 2. Repositório apt da Docker (chave GPG + deb822).
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates curl
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL "https://download.docker.com/linux/$DOCKER_DIST/gpg" -o /etc/apt/keyrings/docker.asc \
        || die "não consegui baixar a chave GPG da Docker"
    chmod a+r /etc/apt/keyrings/docker.asc
    cat > /etc/apt/sources.list.d/docker.sources <<DOCKERSRC
Types: deb
URIs: https://download.docker.com/linux/$DOCKER_DIST
Suites: $CODENAME
Components: stable
Architectures: $(dpkg --print-architecture)
Signed-By: /etc/apt/keyrings/docker.asc
DOCKERSRC
    apt-get update || die "não consegui ler o repositório da Docker (suites '$CODENAME' pode não ser suportada; veja https://docs.docker.com/engine/install/$DOCKER_DIST/)"

    # 3. Os pacotes.
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
        docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin \
        || die "não consegui instalar o docker-ce"

    # 4. Ligado e conferido.
    if [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then
        systemctl enable --now docker >/dev/null 2>&1 || warn "não consegui ativar o serviço docker"
    fi
    command -v docker >/dev/null 2>&1 && say "Docker instalado: $(docker --version)"
}

# ── Podman (opcional: --engine podman) ───────────────────────────────────────

# O Podman não tem daemon residente (o dockerd + containerd custam ~100 MB de RAM parados).
# O rustployd fala com ele pela API compatível com Docker, no socket do podman.socket.

# Algum provider do `podman compose` (é ele que de fato executa o docker-compose.yml)?
has_compose_provider() {
    command -v docker-compose >/dev/null 2>&1 && return 0
    for d in /usr/local/lib/docker/cli-plugins /usr/local/libexec/docker/cli-plugins \
             /usr/lib/docker/cli-plugins /usr/libexec/docker/cli-plugins; do
        [ -x "$d/docker-compose" ] && return 0
    done
    return 1
}

# Candidato instalável de um pacote apt (vazio se o pacote não existe nos repositórios).
apt_candidate() {
    apt-cache policy "$1" 2>/dev/null | awk '/Candidate:/ && $2 != "(none)" {print $2}'
}

install_podman() {
    say "Preparando o Podman (no lugar do Docker)..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y podman || die "não consegui instalar o podman pelo apt"
    # `podman compose` só existe a partir da 4.7 (Debian 12 traz a 4.3 — use Debian 13+/Ubuntu 24.04+).
    PV="$(podman --version | awk '{print $3}')"
    if [ "$(printf '%s\n%s\n' "4.7" "$PV" | sort -V | head -n 1)" != "4.7" ]; then
        die "podman $PV é antigo demais: o rustploy precisa do 'podman compose' (>= 4.7). Use Debian 13+ ou Ubuntu 24.04+, ou fique no Docker."
    fi
    if ! has_compose_provider; then
        say "Instalando um provider para o 'podman compose'..."
        for pkg in docker-compose-v2 docker-compose-plugin docker-compose; do
            [ -n "$(apt_candidate "$pkg")" ] || continue
            if DEBIAN_FRONTEND=noninteractive apt-get install -y "$pkg"; then break; fi
        done
        has_compose_provider || die "não achei nenhum provider de compose (docker-compose-v2, docker-compose-plugin ou docker-compose). Instale um e rode de novo."
    fi
}

# Depois do .deb (que cria o usuário e o grupo rustploy): liga o socket, libera o acesso
# do usuário rustploy, troca o unit para depender do Podman e aponta o config.toml.
configure_podman() {
    say "Configurando o rustployd para usar o Podman..."

    # O socket nasce root:root (0660). O usuário rustploy precisa do grupo dele; num drop-in
    # isso vale para sempre (um chgrp à mão se perde quando o systemd recria o socket).
    mkdir -p /etc/systemd/system/podman.socket.d /etc/systemd/system/rustployd.service.d
    cat > /etc/systemd/system/podman.socket.d/rustploy.conf <<'UNIT'
# Gerado por install.sh --engine podman: deixa o usuário rustploy falar com o socket.
[Socket]
SocketGroup=rustploy
SocketMode=0660
UNIT

    # O unit original manda esperar (e ligar) o docker.service: aqui isso vira o podman.socket.
    # Os "=" vazios zeram a lista do unit original antes de definir a nova.
    cat > /etc/systemd/system/rustployd.service.d/podman.conf <<'UNIT'
# Gerado por install.sh --engine podman: o motor é o Podman, não o Docker.
[Unit]
After=
Wants=
After=network.target podman.socket
Wants=podman.socket

[Service]
Environment=RUSTPLOY_CONTAINER_ENGINE=podman
UNIT

    # [docker] socket_path -> socket do Podman (a chave mantém o nome por compatibilidade).
    if [ -f "$CONFIG" ]; then
        if awk '/^\[/{s=($0=="[docker]")} s && /^socket_path[ \t]*=/{f=1} END{exit !f}' "$CONFIG"; then
            awk -v p="$PODMAN_SOCKET" '/^\[/{s=($0=="[docker]")} s && /^socket_path[ \t]*=/{print "socket_path = \"" p "\""; next} {print}' "$CONFIG" > "$CONFIG.tmp" \
                && cat "$CONFIG.tmp" > "$CONFIG" && rm -f "$CONFIG.tmp"
        else
            printf '\n[docker]\nsocket_path = "%s"\n' "$PODMAN_SOCKET" >> "$CONFIG"
        fi
        say "$CONFIG: [docker] socket_path = \"$PODMAN_SOCKET\""
    else
        warn "$CONFIG não existe; aponte [docker] socket_path para $PODMAN_SOCKET à mão."
    fi

    if [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then
        systemctl daemon-reload || true
        systemctl enable --now podman.socket || warn "não consegui ativar o podman.socket"
        # O socket já existia (de uma execução anterior): o drop-in só vale na próxima ativação.
        systemctl restart podman.socket || true
    fi

    if [ -S /var/run/docker.sock ]; then
        warn "há um Docker instalado nesta máquina. Containers e imagens dele NÃO são movidos para o Podman:"
        warn "faça um novo deploy dos serviços (e recrie bancos/volumes, se for o caso). Depois, 'systemctl disable --now docker' libera a RAM."
    fi
}

# ── Instalar ──────────────────────────────────────────────────────────────────

# Com o motor já instalado, a dependência "docker.io | docker-ce | podman" do .deb fica
# satisfeita e o apt não puxa outro.
if [ "$WHAT" = "daemon" ]; then
    [ "$ENGINE" = "podman" ] && install_podman
    [ "$ENGINE" = "docker" ] && install_docker
fi

say "Instalando (apt resolve as dependências, inclusive o motor de containers, se faltar)..."
# --force-confold: numa atualização, mantém o config.toml (com o token) em vez de perguntar.
DEBIAN_FRONTEND=noninteractive apt-get install -y -o Dpkg::Options::=--force-confold "$TMP/$DEB"

# O clone de serviços com fonte Git usa o git do servidor, e o pacote não depende dele.
if [ "$WHAT" = "daemon" ] && ! command -v git >/dev/null 2>&1; then
    say "Instalando o git (usado nos serviços com fonte Git)..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y git
fi

if [ "$WHAT" = "gui" ]; then
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database /usr/share/applications || true
    say "Pronto. Abra o Rustploy no menu de aplicativos, ou rode: rustploy-gui"
    say "Na tela de login, cole o token de conexão do seu servidor (veja $DOCS/primeiro-acesso/)."
    exit 0
fi

# ── Daemon: token, subir e mostrar o token ───────────────────────────────────

# O pacote vem sem [api] (loopback e SEM token), o que não deixa nenhum cliente entrar.
# Aqui: gera um token forte e grava o [api]. Não mexe se o config.toml já tem [api].
setup_api() {
    [ -f "$CONFIG" ] || { warn "$CONFIG não existe; não configurei o token."; return 0; }
    if grep -q '^\[api\]' "$CONFIG"; then
        # Já configurado (outra execução, ou à mão): o token nunca é trocado. Só o --remote
        # pode abrir uma API que está em loopback.
        if [ "$REMOTE" = "1" ] && awk '/^\[/{s=($0=="[api]")} s && /^bind_address[ \t]*=[ \t]*"127\.0\.0\.1"/{f=1} END{exit !f}' "$CONFIG"; then
            if ! awk '/^\[/{s=($0=="[api]")} s && /^token[ \t]*=/{f=1} END{exit !f}' "$CONFIG"; then
                warn "o [api] de $CONFIG não tem token; o daemon recusa bind público sem token. Não alterei."
                return 0
            fi
            awk '/^\[/{s=($0=="[api]")} s && /^bind_address[ \t]*=/{print "bind_address = \"0.0.0.0\""; next} {print}' "$CONFIG" > "$CONFIG.tmp" \
                && cat "$CONFIG.tmp" > "$CONFIG" && rm -f "$CONFIG.tmp"
            say "A API agora escuta em 0.0.0.0:9797 (o token existente foi mantido)."
        else
            say "$CONFIG já tem [api]; não alterei."
        fi
        return 0
    fi
    TOKEN="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    BIND="127.0.0.1"
    [ "$REMOTE" = "1" ] && BIND="0.0.0.0"
    cat >> "$CONFIG" <<CFG

# Acesso à API com token (gerado por install.sh).
[api]
bind_address = "$BIND"
port         = 9797
token        = "$TOKEN"
CFG
    say "Token da API gerado e gravado em $CONFIG (API em $BIND:9797)."
}
setup_api
[ "$ENGINE" = "podman" ] && configure_podman

if [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload || true
    systemctl enable --now rustployd-fw.socket >/dev/null 2>&1 || warn "não consegui ativar rustployd-fw.socket (liberação automática de porta externa)"
    systemctl enable --now rustployd >/dev/null 2>&1 || true
    systemctl restart rustployd || true
    i=0
    while [ "$i" -lt 20 ]; do
        systemctl is-active --quiet rustployd && break
        i=$((i + 1)); sleep 1
    done
    if systemctl is-active --quiet rustployd; then
        say "rustployd está ativo."
    else
        warn "rustployd não subiu. Veja: journalctl -u rustployd -n 50 --no-pager"
    fi
else
    warn "systemd não está ativo nesta máquina; o serviço não foi iniciado."
fi

say "Token de conexão (cole na GUI, na web UI ou no MCP):"
rustployd access-key || warn "não consegui ler o token. Rode: sudo rustployd access-key"

if [ "$REMOTE" = "1" ]; then
    cat <<END

Próximos passos:
  1. Abra a porta 9797 (API e web UI) no firewall da VPS; para domínios, também 80 e 443.
  2. Abra http://IP_DA_VPS:9797 no navegador e cole o token acima.
  3. Sem HTTPS, o token trafega sem cifra: configure um domínio assim que puder.
     Guia completo: $DOCS/instalar-daemon/
END
else
    cat <<END

A API escuta só em 127.0.0.1 (nada exposto à internet). Para usar daqui:
  ssh -N -L 9797:127.0.0.1:9797 USUARIO@IP_DA_VPS     (e abra http://localhost:9797 no seu navegador)
Para acessar direto de outra máquina (GUI desktop, web UI, MCP), rode de novo com --remote:
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sudo sh -s -- --remote
Guia completo: $DOCS/instalar-daemon/
END
fi
