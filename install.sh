#!/bin/sh
# Instalador do Rustploy (Debian, Ubuntu e derivados), a partir da última release do GitHub.
#
#   Daemon, na VPS:    curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh
#   App desktop:       curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh -s -- gui
#
# Opções (depois de `sh -s --`, ou direto: `sudo sh install.sh gui --version v0.1.1`):
#   daemon | gui        o que instalar (padrão: daemon)
#   engine              só escolhe/troca o motor de containers de um rustployd JÁ instalado (usado
#                       pelo `make install-rustploy`); não baixa nem instala pacote do rustploy
#   --remote            (daemon) a API aceita conexões de fora da VPS (bind 0.0.0.0, com token).
#                       Sem isso, ela escuta só em 127.0.0.1 (use um túnel SSH ou rode de novo com --remote).
#   --engine docker|podman  o motor de containers. Sem a opção, PERGUNTA (se houver terminal) e o padrão é o
#                       motor que o rustploy já usa (ou docker, numa instalação nova). Em ambos o
#                       instalador instala o motor se faltar: docker = Docker Engine pelo repositório
#                       oficial da Docker; podman = Podman + compose, podman.socket, acesso do usuário
#                       rustploy e config.toml. Docker -> Podman numa VPS que já roda serviços MIGRA:
#                       copia os volumes (sem apagar o original) e refaz o deploy dos serviços.
#   --yes | -y          confirma a troca de motor sem perguntar (para scripts)
#   --deb ARQUIVO       instala um .deb local (ex.: make deb-rustploy) em vez de baixar a release
#   --version vX.Y.Z    uma release específica (padrão: a última)
#   --dry-run           só mostra o que baixaria e instalaria
#   -h | --help         esta ajuda
#
# Variáveis: RUSTPLOY_VERSION (igual a --version), RUSTPLOY_ENGINE (igual a --engine), RUSTPLOY_YES=1, RUSTPLOY_DRY_RUN=1,
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
ENGINE="${RUSTPLOY_ENGINE:-}"
YES="${RUSTPLOY_YES:-0}"
SWITCH=0
CURRENT=""
REDEPLOY_FILE=""
LOCAL_DEB=""
PODMAN_SOCKET="/run/podman/podman.sock"
ENGINE_CHOICE="/etc/rustploy/engine"
CONFIG="/etc/rustploy/config.toml"

say() { printf '==> %s\n' "$*"; }
warn() { printf 'aviso: %s\n' "$*" >&2; }
die() { printf 'erro: %s\n' "$*" >&2; exit 1; }

# ── Escolha do motor de containers ────────────────────────────────────────────

# Valor de uma chave de uma seção do config.toml (vazio se não houver).
cfg_get() {
    [ -r "$CONFIG" ] || return 0
    awk -v sec="[$1]" -v key="$2" '/^\[/{s=($0==sec)} s && $1==key {sub(/^[^=]*=[ \t]*/,""); gsub(/"/,""); print; exit}' "$CONFIG"
}

# Motor que o rustploy usa hoje: docker | podman | vazio (instalação nova, sem config).
current_engine() {
    [ -r "$CONFIG" ] || return 0
    case "$(cfg_get docker socket_path)" in
        "") echo "" ;;
        *podman*) echo podman ;;
        *) echo docker ;;
    esac
}

# Há um terminal para perguntar? Com `curl | sh` o stdin é o próprio script, então a
# pergunta lê de /dev/tty.
can_prompt() { [ -r /dev/tty ] && ( : </dev/tty ) 2>/dev/null; }

# ask "pergunta" "padrão" -> resposta em $ANSWER (lida do terminal)
ask() {
    printf '%s ' "$1" > /dev/tty
    read -r ANSWER < /dev/tty || ANSWER=""
    [ -n "$ANSWER" ] || ANSWER="$2"
}


# TODOS os volumes do Docker que vale copiar: os nomeados (com ou sem container) e os anônimos
# ainda em uso. Fica de fora só o lixo: volume anônimo (nome de 64 hex) sem nenhum container.
vol_is_dangling_anon() {
    case "$1" in
        [0-9a-f]*) [ "${#1}" -eq 64 ] && [ -z "$(docker ps -aq --filter "volume=$1" 2>/dev/null)" ] ;;
        *) return 1 ;;
    esac
}
docker_vols_to_copy() {
    for _v in $(docker volume ls -q 2>/dev/null); do vol_is_dangling_anon "$_v" || echo "$_v"; done
}

# Redes criadas por usuário/compose/rustploy (exclui bridge, host e none).
docker_custom_networks() { docker network ls --filter type=custom --format '{{.Name}}' 2>/dev/null; }

# Imagens com nome (sem <none>), uma referência por linha.
docker_image_refs() { docker images --format '{{.Repository}}:{{.Tag}}' 2>/dev/null | grep -v '<none>' | sort -u; }

# Nome completo que o Podman usa para uma referência curta do Docker (nginx:alpine ->
# docker.io/library/nginx:alpine), igual ao que um build/pull novo gera.
norm_ref() {
    case "$1" in
        */*) case "${1%%/*}" in *.*|*:*|localhost) echo "$1" ;; *) echo "docker.io/$1" ;; esac ;;
        *) echo "docker.io/library/$1" ;;
    esac
}

# Tamanho de um diretório em KB.
dir_kb() { du -sk "$1" 2>/dev/null | cut -f1; }

# Decide ENGINE (pergunta se não foi dado) e se isso é uma TROCA docker -> podman (SWITCH=1).
choose_engine() {
    CURRENT="$(current_engine)"
    # Quem escolheu "Docker" já foi perguntado: lembra a escolha para não repetir a pergunta a cada
    # atualização. Para ser perguntado de novo, apague o arquivo (ou passe --engine).
    if [ -z "$ENGINE" ] && [ -r "$ENGINE_CHOICE" ]; then
        ENGINE="$(head -n 1 "$ENGINE_CHOICE")"
        case "$ENGINE" in docker) say "Motor de containers: docker (escolha salva em $ENGINE_CHOICE; apague o arquivo para ser perguntado de novo).";; *) ENGINE="";; esac
    fi
    if [ -z "$ENGINE" ]; then
        DEFAULT="${CURRENT:-docker}"
        if can_prompt; then
            [ -n "$CURRENT" ] && printf '\nO rustploy desta máquina usa hoje: %s\n' "$CURRENT" > /dev/tty
            cat > /dev/tty <<'ASK'

Qual motor de containers usar?
  1) Docker  - o padrão; dockerd + containerd ficam sempre rodando (~150 MB de RAM parados)
  2) Podman  - sem daemon residente: economiza ~150 MB de RAM parado (precisa de Debian 13+/Ubuntu 24.04+)
ASK
            [ "$DEFAULT" = "podman" ] && D=2 || D=1
            ask "Escolha [1/2] (padrão $D):" "$D"
            case "$ANSWER" in 2|podman|Podman) ENGINE=podman ;; *) ENGINE=docker ;; esac
            # Escolheu continuar no Docker: grava para não perguntar de novo (só com o rustploy já instalado).
            if [ "$ENGINE" = "docker" ] && [ -n "$CURRENT" ] && [ "$DRY" != "1" ] && [ -d /etc/rustploy ]; then
                echo docker > "$ENGINE_CHOICE" 2>/dev/null || true
            fi
        else
            ENGINE="$DEFAULT"
        fi
    fi
    if [ "$CURRENT" = "podman" ] && [ "$ENGINE" = "docker" ]; then
        die "este rustploy já usa o Podman; voltar ao Docker não é automático (apague /etc/systemd/system/rustployd.service.d/podman.conf e aponte [docker] socket_path de $CONFIG para /var/run/docker.sock)."
    fi
    if [ "$CURRENT" = "docker" ] && [ "$ENGINE" = "podman" ]; then
        SWITCH=1
        confirm_switch
    fi
}

# Mostra o que a troca vai fazer e pede confirmação (sem terminal: exige --yes).
confirm_switch() {
    if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
        say "Troca Docker -> Podman: o Docker não responde (ou não está instalado); não há o que migrar."
        return 0
    fi
    NC="$(docker ps -aq --filter 'name=^rp_' | wc -l | tr -d ' ')"
    NR="$(docker ps -q --filter 'name=^rp_' | wc -l | tr -d ' ')"
    VOLS="$(docker_vols_to_copy)"
    NV="$(printf '%s\n' "$VOLS" | grep -c . || true)"
    NETS="$(docker_custom_networks)"
    NN="$(printf '%s\n' "$NETS" | grep -c . || true)"
    REFS="$(docker_image_refs)"
    NI="$(printf '%s\n' "$REFS" | grep -c . || true)"
    say "Troca Docker -> Podman. TUDO do Docker é reaproveitado no Podman; o que vai acontecer:"
    say "  1. backup de /etc/rustploy e do banco do rustploy em /var/backups/"
    say "  2. transferir $NI imagem(ns) do Docker para o Podman (sem parar nada; o Docker fica intacto)"
    say "  3. parar o rustployd e os $NR container(s) rodando de $NC do rustploy (o Docker NÃO é desligado nem apagado)"
    say "  4. copiar $NV volume(s) (cópia: o original fica intacto):"
    for v in $VOLS; do
        say "       - $v ($(du -sh "$(docker volume inspect -f '{{.Mountpoint}}' "$v" 2>/dev/null)" 2>/dev/null | cut -f1))"
    done
    say "  5. recriar $NN rede(s) criadas por usuário/compose (as do próprio rustploy são recriadas no deploy)"
    say "  6. apontar o rustploy para o Podman e refazer o deploy dos serviços que estavam rodando"
    say "     (ficam indisponíveis durante a cópia e o deploy; o cache de build é refeito)"
    [ "$DRY" = "1" ] && return 0
    if [ "$YES" = "1" ]; then return 0; fi
    can_prompt || die "a troca de motor precisa de confirmação: rode de novo com --yes (ou num terminal)."
    ask "Continuar? [s/N]:" "N"
    case "$ANSWER" in s|S|sim|Sim|y|Y|yes) ;; *) die "troca cancelada; nada foi alterado." ;; esac
}

usage() {
    cat <<'HELP'
Instalador do Rustploy (Debian, Ubuntu e derivados), a partir da última release do GitHub.

Uso: sudo sh install.sh [daemon|gui|engine] [--remote] [--yes] [--engine docker|podman] [--version vX.Y.Z] [--dry-run]

  daemon            instala o rustployd na VPS (padrão)
  gui               instala o app desktop (amd64)
  --remote          (daemon) a API aceita conexões de fora da VPS (bind 0.0.0.0, com token);
                    sem isso, escuta só em 127.0.0.1
  engine            só escolhe/troca o motor de um rustployd já instalado (sem baixar pacote)
  --engine docker   motor de containers = Docker; instala o Docker Engine se faltar
  --engine podman   motor de containers = Podman; instala e configura tudo. Numa VPS que já roda
                    em Docker, MIGRA (copia os volumes, refaz o deploy). Sem --engine, pergunta.
  --yes, -y         confirma a troca de motor sem perguntar
  --deb ARQUIVO     instala um .deb local em vez de baixar a release (teste de um build seu)
  --version vX.Y.Z  uma release específica (padrão: a última)
  --dry-run         só mostra o que baixaria e instalaria
HELP
}

while [ $# -gt 0 ]; do
    case "$1" in
        daemon|gui|engine) WHAT="$1" ;;
        --version) shift; [ $# -gt 0 ] || die "--version precisa de um valor (ex.: v0.1.1)"; VERSION="$1" ;;
        --version=*) VERSION="${1#--version=}" ;;
        --dry-run) DRY=1 ;;
        --yes|-y) YES=1 ;;
        --remote) REMOTE=1 ;;
        --deb) shift; [ $# -gt 0 ] || die "--deb precisa de um arquivo"; LOCAL_DEB="$1" ;;
        --deb=*) LOCAL_DEB="${1#--deb=}" ;;
        --engine) shift; [ $# -gt 0 ] || die "--engine precisa de um valor (docker ou podman)"; ENGINE="$1" ;;
        --engine=*) ENGINE="${1#--engine=}" ;;
        -h|--help) usage; exit 0 ;;
        *) die "argumento desconhecido: $1 (use --help)" ;;
    esac
    shift
done

case "$ENGINE" in
    ""|docker) ;;
    podman) [ "$WHAT" = "gui" ] && die "--engine podman vale só para o daemon" ;;
    *) die "--engine inválido: $ENGINE (use docker ou podman)" ;;
esac
[ "$WHAT" = "engine" ] && [ "$ENGINE" = "" ] && [ ! -r /dev/tty ] && die "engine: sem terminal para perguntar; passe --engine docker ou --engine podman"

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

# Daemon/engine: decide o motor (pergunta, se preciso) ANTES de baixar qualquer coisa.
[ "$WHAT" = "gui" ] || choose_engine
[ "$DRY" = "1" ] && [ "$WHAT" = "engine" ] && exit 0

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
    daemon:amd64|daemon:arm64|gui:amd64|engine:*) ;;
    gui:*) die "o app desktop só tem pacote para amd64 por enquanto (esta máquina é $ARCH)" ;;
    *) die "arquitetura sem pacote: $ARCH (suportadas: amd64 e arm64)" ;;
esac

case "$WHAT" in
    daemon|engine) PREFIX="rustployd_" ;;
    gui)    PREFIX="rustploy-gui_" ;;
esac

# ── Resolver a release ────────────────────────────────────────────────────────

# --deb ARQUIVO: instala um .deb local (ex.: gerado por `make deb-rustploy`), sem consultar o GitHub.
# O bloco abaixo (resolver, baixar e conferir) só roda sem --deb; o resto do script é igual.
# No modo `engine` não há pacote nenhum para baixar.
if [ "$WHAT" = "engine" ]; then
:
elif [ -n "$LOCAL_DEB" ]; then
[ -f "$LOCAL_DEB" ] || die "--deb: arquivo não encontrado: $LOCAL_DEB"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM
DEB="${LOCAL_DEB##*/}"
say "Usando o pacote local $LOCAL_DEB (sem consultar o GitHub, sem conferir SHA-256)."
if [ "$DRY" = "1" ]; then
    say "--dry-run: instalaria com: apt-get install -y $LOCAL_DEB"
    exit 0
fi
cp "$LOCAL_DEB" "$TMP/$DEB"
else
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

    # Acrescenta o podman.socket às dependências do unit (que só ordena depois do docker.service,
    # sem puxá-lo). Atenção: zerar `Wants=` com "=" vazio num drop-in NÃO remove o que o unit
    # original declara (testado), por isso o unit do pacote já não tem `Wants=docker.service`.
    cat > /etc/systemd/system/rustployd.service.d/podman.conf <<'UNIT'
# Gerado por install.sh --engine podman: o motor é o Podman, não o Docker.
[Unit]
After=podman.socket
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

    if [ "$SWITCH" != "1" ] && [ -S /var/run/docker.sock ]; then
        warn "há um Docker instalado nesta máquina. Containers e imagens dele NÃO são movidos para o Podman:"
        warn "faça um novo deploy dos serviços (e recrie bancos/volumes, se for o caso). Depois, 'systemctl disable --now docker' libera a RAM."
    fi
}


# ── Migração Docker -> Podman (VPS que já roda serviços) ──────────────────────

# Chamada à API do próprio rustployd (/api/rpc), pelo token do config.toml. Tenta http e depois
# https (com [api].domain a porta fala TLS). Corpo JSON em $1; resposta em stdout.
rp_api() {
    _port="$(cfg_get api port)"; _port="${_port:-9797}"
    _token="$(cfg_get api token)"
    for _scheme in http https; do
        _out="$(curl -sk --max-time 30 -H "Authorization: Bearer $_token" -d "$1" "$_scheme://127.0.0.1:$_port/api/rpc")" \
            && [ -n "$_out" ] && { printf '%s' "$_out"; return 0; }
    done
    return 1
}

# Ids dos serviços que estavam no ar (Running ou Degraded), um por linha. Precisa de python3
# para ler o JSON; sem ele devolve erro e o deploy fica por conta do usuário.
running_service_ids() {
    command -v python3 >/dev/null 2>&1 || return 1
    _projects="$(rp_api '"ProjectList"')" || return 1
    for _p in $(printf '%s' "$_projects" | python3 -c 'import sys,json; print(" ".join(p["id"] for p in json.load(sys.stdin)["Projects"]))'); do
        _svcs="$(rp_api "{\"ServiceList\":{\"project_id\":\"$_p\"}}")" || continue
        printf '%s' "$_svcs" | python3 -c '
import sys, json
for s in json.load(sys.stdin).get("Services", []):
    if s.get("status") in ("Running", "Degraded"):
        print(s["id"])'
    done
}

# Desfaz uma migração que falhou ANTES de trocar o motor: sobe de novo os containers parados e o
# rustployd, ainda no Docker. Nada do Docker foi apagado, então é só religar.
rollback_docker() {
    warn "falhou: voltando ao Docker (religando o que foi parado)..."
    if [ -s "$STOPPED_FILE" ]; then
        # shellcheck disable=SC2046
        docker start $(cat "$STOPPED_FILE") >/dev/null 2>&1 || true
    fi
    systemctl start rustployd >/dev/null 2>&1 || true
    die "migração para o Podman interrompida; o rustploy segue no Docker. Os volumes já copiados ficam no Podman (inofensivos)."
}

# Passos da troca (o rustployd ainda roda no Docker quando isto começa). NÃO reinicia o rustployd
# nem refaz o deploy: quem chamou faz isso (finish_switch) depois de reiniciá-lo.
migrate_to_podman() {
    say "Migrando do Docker para o Podman..."
    command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1 || {
        say "O Docker não responde: nada a migrar, só apontando o rustploy para o Podman."
        configure_podman
        return 0
    }

    # (1) o que estava no ar, ANTES de parar tudo (a API só responde com o rustployd de pé)
    REDEPLOY_FILE="$(mktemp)"
    if running_service_ids > "$REDEPLOY_FILE" 2>/dev/null; then
        say "Serviços que estavam no ar: $(grep -c . "$REDEPLOY_FILE" || true)"
    else
        warn "não consegui listar os serviços no ar (precisa de python3 e do rustployd respondendo); refaça o deploy à mão depois."
        : > "$REDEPLOY_FILE"
    fi

    # (2) backup do que é do rustploy (o registry embutido, que pode ser grande, fica de fora)
    BK="/var/backups/rustploy-antes-do-podman-$(date +%Y%m%d-%H%M%S).tar.gz"
    mkdir -p /var/backups
    DBP="$(cfg_get daemon db_path)"; DBP="${DBP:-/var/lib/rustploy/db}"
    tar czf "$BK" --exclude="$DBP/registry" /etc/rustploy "$DBP" 2>/dev/null || warn "o backup teve avisos (arquivos que mudaram durante a cópia)"
    chmod 600 "$BK"
    say "Backup: $BK"

    # (3) espaço no destino: volumes + imagens (com 10% de folga)
    VOLS="$(docker_vols_to_copy)"
    REFS="$(docker_image_refs)"
    NEED=0
    for v in $VOLS; do NEED=$((NEED + $(dir_kb "$(docker volume inspect -f '{{.Mountpoint}}' "$v")" || echo 0))); done
    if [ -n "$REFS" ]; then
        # shellcheck disable=SC2086
        IMGB="$(docker image inspect -f '{{.Size}}' $REFS 2>/dev/null | awk '{s+=$1} END{print s+0}')"
        NEED=$((NEED + IMGB / 1024))
    fi
    GR="$(podman info --format '{{.Store.GraphRoot}}' 2>/dev/null)"; GR="${GR:-/var/lib/containers/storage}"
    mkdir -p "$GR"
    FREE="$(df -Pk "$GR" | awk 'NR==2{print $4}')"
    if [ "$NEED" -gt 0 ] && [ "${FREE:-0}" -lt $((NEED + NEED / 10)) ]; then
        die "espaço insuficiente em $GR para copiar volumes e imagens (precisa de ~$((NEED / 1024)) MB, há $((FREE / 1024)) MB). Nada foi alterado."
    fi

    # (4) imagens: NÃO exige parar nada, então vai primeiro (menos tempo fora do ar). O `podman load`
    # nomeia as de nome curto como localhost/...; cada uma ganha também o nome completo
    # (docker.io/library/...) que um build/pull novo geraria. Se falhar, só se perde cache.
    if [ -n "$REFS" ]; then
        say "Transferindo $(printf '%s\n' "$REFS" | grep -c .) imagem(ns) para o Podman (pode levar um tempo)..."
        # shellcheck disable=SC2086
        if docker save $REFS | podman load >/dev/null 2>&1; then
            for r in $REFS; do
                q="$(norm_ref "$r")"
                [ "$q" = "$r" ] || podman tag "localhost/$r" "$q" >/dev/null 2>&1 || true
            done
        else
            warn "não consegui transferir as imagens; elas serão baixadas/construídas de novo no deploy (rollback para versões antigas só depois de um novo deploy)."
        fi
    fi

    # (5) parar: primeiro o rustployd (não deixa nada religar), depois os containers do rustploy
    STOPPED_FILE="$(mktemp)"
    systemctl stop rustployd >/dev/null 2>&1 || true
    docker ps -q --filter 'name=^rp_' > "$STOPPED_FILE"
    if [ -s "$STOPPED_FILE" ]; then
        say "Parando $(grep -c . "$STOPPED_FILE") container(s) do rustploy no Docker (para os dados ficarem consistentes)..."
        # shellcheck disable=SC2046
        docker stop -t 30 $(cat "$STOPPED_FILE") >/dev/null || rollback_docker
    fi
    # Sem religar sozinhos se alguém reiniciar o Docker (política de reinício do compose do usuário).
    # shellcheck disable=SC2046
    docker update --restart=no $(docker ps -aq --filter 'name=^rp_') >/dev/null 2>&1 || true

    # (6) volumes: mesmo nome, mesmos labels. Volume com opções (NFS, bind...) é RECRIADO com as mesmas
    # opções, sem copiar (os dados moram fora do volume). Os demais são copiados com cp -a.
    for v in $VOLS; do
        drv="$(docker volume inspect -f '{{.Driver}}' "$v")"
        if [ "$drv" != "local" ]; then
            warn "volume $v usa o driver '$drv', que o Podman não tem: NÃO foi copiado."
            continue
        fi
        if podman volume exists "$v" 2>/dev/null; then
            dst="$(podman volume inspect -f '{{.Mountpoint}}' "$v")"
            if [ -n "$(ls -A "$dst" 2>/dev/null)" ]; then
                warn "volume $v já existe no Podman e tem dados; mantido, NÃO copiei por cima."
                continue
            fi
        else
            LABELS="$(docker volume inspect -f '{{range $k, $x := .Labels}}--label {{$k}}={{$x}} {{end}}' "$v")"
            OPTS="$(docker volume inspect -f '{{range $k, $x := .Options}}-o {{$k}}={{$x}} {{end}}' "$v")"
            # shellcheck disable=SC2086
            podman volume create $LABELS $OPTS "$v" >/dev/null || rollback_docker
            if [ -n "$OPTS" ]; then
                say "Volume $v recriado com as mesmas opções ($OPTS)."
                continue
            fi
            dst="$(podman volume inspect -f '{{.Mountpoint}}' "$v")"
        fi
        # Um container que NÃO é do rustploy ainda usando o volume = cópia "a quente".
        [ -z "$(docker ps -q --filter "volume=$v" 2>/dev/null)" ] \
            || warn "o volume $v está em uso por um container que não é do rustploy; a cópia pode ficar inconsistente."
        src="$(docker volume inspect -f '{{.Mountpoint}}' "$v")"
        say "Copiando o volume $v..."
        cp -a "$src/." "$dst/" || rollback_docker
    done

    # (7) redes de usuário/compose: mesmo nome e labels. Sub-rede automática (a do Docker continua
    # ocupada enquanto ele rodar). Só o driver bridge; as do rustploy (rp_net_*) o deploy recria.
    for n in $(docker_custom_networks); do
        podman network exists "$n" 2>/dev/null && continue
        [ "$(docker network inspect -f '{{.Driver}}' "$n")" = "bridge" ] || { warn "rede $n não é bridge; não recriada."; continue; }
        FLAGS=""
        [ "$(docker network inspect -f '{{.Internal}}' "$n")" = "true" ] && FLAGS="$FLAGS --internal"
        [ "$(docker network inspect -f '{{.EnableIPv6}}' "$n")" = "true" ] && FLAGS="$FLAGS --ipv6"
        LABELS="$(docker network inspect -f '{{range $k, $x := .Labels}}--label {{$k}}={{$x}} {{end}}' "$n")"
        # shellcheck disable=SC2086
        podman network create $FLAGS $LABELS "$n" >/dev/null 2>&1 && say "Rede $n recriada." || warn "não consegui recriar a rede $n."
    done

    configure_podman
}

# Depois do rustployd reiniciado (já no Podman): espera a API e refaz o deploy dos serviços.
finish_switch() {
    [ -n "$REDEPLOY_FILE" ] && [ -s "$REDEPLOY_FILE" ] || {
        say "Troca concluída. Refaça o deploy dos seus serviços (não consegui listá-los automaticamente)."
        return 0
    }
    i=0
    while [ "$i" -lt 30 ]; do rp_api '"ProjectList"' >/dev/null 2>&1 && break; i=$((i + 1)); sleep 1; done
    say "Refazendo o deploy dos serviços que estavam no ar..."
    for id in $(cat "$REDEPLOY_FILE"); do
        rp_api "{\"DeployStart\":{\"service_id\":\"$id\"}}" >/dev/null 2>&1 \
            && say "  deploy enfileirado: $id" || warn "  não consegui enfileirar $id; faça o deploy à mão."
    done
    say "Troca concluída. Acompanhe os deploys na GUI ou na web UI. O Docker segue instalado e intacto:"
    say "depois de conferir que tudo voltou, libere a RAM com: sudo systemctl disable --now docker docker.socket"
}

# Modo `engine`: só escolhe/troca o motor de um rustployd já instalado.
engine_only() {
    command -v rustployd >/dev/null 2>&1 || die "rustployd não está instalado; rode sem o modo 'engine' para instalar."
    if [ "$ENGINE" = "podman" ] && [ "$SWITCH" = "1" ]; then
        install_podman
        migrate_to_podman
        systemctl daemon-reload || true
        systemctl restart rustployd || true
        finish_switch
    elif [ "$ENGINE" = "podman" ] && [ -z "$CURRENT" ]; then
        install_podman
        configure_podman
        systemctl restart rustployd || true
    elif [ "$ENGINE" = "docker" ]; then
        install_docker
        say "Motor: Docker (sem mudanças)."
    else
        say "Motor: $ENGINE (sem mudanças)."
    fi
}

# ── Instalar ──────────────────────────────────────────────────────────────────

if [ "$WHAT" = "engine" ]; then engine_only; exit 0; fi

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
if [ "$ENGINE" = "podman" ]; then
    if [ "$SWITCH" = "1" ]; then migrate_to_podman; else configure_podman; fi
fi

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
        [ "$SWITCH" = "1" ] && finish_switch
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
