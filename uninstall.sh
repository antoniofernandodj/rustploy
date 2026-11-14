#!/bin/sh
# Desinstalador do Rustploy (Debian, Ubuntu e derivados): o inverso do install.sh.
#
#   curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/uninstall.sh | sudo sh
#   ou, com opções:   sudo sh uninstall.sh --dry-run
#
# O QUE REMOVE (tudo o que é do rustploy, e só isso):
#   - os serviços que ele criou: containers `rp_*`, volumes `rp_*`, redes `rp_net_*` e imagens `rp_*`
#     (no Podman e/ou no Docker, o que estiver nesta máquina);
#   - os pacotes rustployd, rustploy-gui e rustploy (purge: leva o /etc/rustploy/config.toml);
#   - os dados do daemon (/var/lib/rustploy: banco, registry embutido) e /etc/rustploy
#     (inclui a identidade age, age.key);
#   - os units e drop-ins do systemd, as configs do Podman que o instalador gravou
#     (/etc/containers/*.d/rustploy.conf) e as regras do ufw (comentário "rustploy");
#   - o usuário e o grupo `rustploy`.
#
# O QUE NÃO REMOVE:
#   - o Docker/Podman em si (outros programas podem usá-lo): só o que é `rp_*`. Imagens de terceiros
#     (postgres, redis...) que os serviços puxaram também ficam; `podman image prune -a` limpa se quiser;
#   - backups em /var/backups/rustploy-* (têm segredos; apague à mão se não precisar);
#   - os dados do app desktop de cada usuário (~/.config/rustploy-gui*.json);
#   - o que está FORA desta máquina: o repositório git do sync e os backups no S3. É daí que a
#     recuperação (Settings → Recuperar) traz tudo de volta, junto com a chave de recuperação.
#
# ANTES DE RODAR: confirme que o git sync e o backup de dados estão em dia e que você TEM o arquivo da
# chave de recuperação (privada). Sem ela, o que está cifrado no git e no S3 não volta.
#
# Opções:
#   --keep-data   desinstala os pacotes e os serviços do systemd, mas MANTÉM os volumes, /var/lib/rustploy
#                 e /etc/rustploy (equivale ao antigo `dpkg -r`, que o install.sh sabe reaproveitar)
#   --yes | -y    não pede a confirmação (para scripts)
#   --dry-run     só mostra o que faria, sem alterar nada
#   -h | --help   esta ajuda
#
# Variáveis: RUSTPLOY_YES=1, RUSTPLOY_DRY_RUN=1.
#
# Windows: o app desktop se remove em "Aplicativos" (ou apagando a pasta onde o install.ps1 o pôs).
set -eu

DRY="${RUSTPLOY_DRY_RUN:-0}"
YES="${RUSTPLOY_YES:-0}"
KEEP_DATA=0

CONFIG="/etc/rustploy/config.toml"
STATE_DIR="/var/lib/rustploy"
ETC_DIR="/etc/rustploy"

say() { printf '==> %s\n' "$*"; }
warn() { printf 'aviso: %s\n' "$*" >&2; }
die() { printf 'erro: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'HELP'
Desinstalador do Rustploy (Debian, Ubuntu e derivados).

Uso: sudo sh uninstall.sh [--keep-data] [--yes] [--dry-run]

  --keep-data   remove os pacotes e os serviços do systemd, mas mantém volumes, /var/lib/rustploy e /etc/rustploy
  --yes, -y     não pede a confirmação
  --dry-run     só mostra o que faria

Remove: serviços (containers/volumes/redes/imagens rp_*), pacotes, dados do daemon, units e drop-ins do
systemd, configs do Podman do instalador, regras do ufw e o usuário rustploy.
Mantém: o Docker/Podman em si, /var/backups/rustploy-*, e o que está fora da máquina (git e S3).
HELP
}

while [ $# -gt 0 ]; do
    case "$1" in
        --keep-data) KEEP_DATA=1 ;;
        --yes|-y) YES=1 ;;
        --dry-run) DRY=1 ;;
        -h|--help) usage; exit 0 ;;
        *) die "argumento desconhecido: $1 (use --help)" ;;
    esac
    shift
done

[ "$(id -u)" = "0" ] || die "precisa de root. Rode: sudo sh uninstall.sh"
command -v dpkg >/dev/null 2>&1 || die "este desinstalador é para Debian/Ubuntu (dpkg). Em outros sistemas, remova os pacotes à mão."

# Executa (ou, no --dry-run, só mostra). Falha de um passo vira aviso: o desinstalador segue até o fim.
run() {
    if [ "$DRY" = "1" ]; then
        printf '   [dry-run] %s\n' "$*"
    else
        "$@" >/dev/null 2>&1 || warn "falhou (seguindo): $*"
    fi
}

have_systemd() { [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; }

# Motores presentes e respondendo. Pode haver os dois (migração Docker → Podman não apaga o original).
ENGINES=""
for e in podman docker; do
    command -v "$e" >/dev/null 2>&1 && "$e" info >/dev/null 2>&1 && ENGINES="$ENGINES $e"
done

# Itens do rustploy em cada motor (um por linha).
list_containers() { "$1" ps -a --format '{{.Names}}' 2>/dev/null | grep '^rp_' || true; }
list_volumes()    { "$1" volume ls --format '{{.Name}}' 2>/dev/null | grep '^rp_' || true; }
list_networks()   { "$1" network ls --format '{{.Name}}' 2>/dev/null | grep '^rp_net_' || true; }
# Imagens que o rustploy construiu: o repositório começa com rp_ (ex.: docker.io/library/rp_api:01M...).
list_images()     { "$1" images --format '{{.ID}} {{.Repository}}' 2>/dev/null | awk '{n=$2; sub(/^.*\//,"",n); if (n ~ /^rp_/) print $1}' | sort -u || true; }

count() { [ -n "$1" ] && printf '%s\n' "$1" | grep -c . || echo 0; }

# ── Resumo e confirmação ──────────────────────────────────────────────────────

PKGS=""
for p in rustployd rustploy-gui rustploy; do
    dpkg -s "$p" >/dev/null 2>&1 && PKGS="$PKGS $p"
done
PKGS="${PKGS# }"

printf '\nIsto vai REMOVER o rustploy desta máquina:\n\n'
[ -n "$PKGS" ] && printf '  pacotes:        %s\n' "$PKGS" || printf '  pacotes:        (nenhum instalado)\n'
if [ "$KEEP_DATA" = "1" ]; then
    printf '  dados:          MANTIDOS (--keep-data): volumes, %s, %s\n' "$STATE_DIR" "$ETC_DIR"
else
    for e in $ENGINES; do
        printf '  %-15s %s containers, %s volumes, %s redes, %s imagens rp_*\n' "$e:" \
            "$(count "$(list_containers "$e")")" "$(count "$(list_volumes "$e")")" \
            "$(count "$(list_networks "$e")")" "$(count "$(list_images "$e")")"
    done
    [ -z "$ENGINES" ] && printf '  containers:     nenhum motor (docker/podman) respondendo; nada a remover lá\n'
    printf '  dados:          %s (banco, registry) e %s (config e age.key)\n' "$STATE_DIR" "$ETC_DIR"
fi
printf '  systemd/ufw:    units, drop-ins, configs do Podman do instalador e regras "rustploy" do ufw\n'
printf '  usuário:        rustploy (e o grupo)\n'
if [ "$KEEP_DATA" != "1" ]; then
    printf '\nO que está FORA desta máquina continua: o repositório git do sync e os backups no S3.\n'
    printf 'Para trazer tudo de volta você vai precisar do arquivo da CHAVE DE RECUPERAÇÃO (privada).\n'
    printf 'Se não tiver, pare agora: sem ela o que está cifrado lá não volta.\n'
fi
printf '\n'

if [ "$DRY" != "1" ] && [ "$YES" != "1" ]; then
    [ -r /dev/tty ] || die "sem terminal para confirmar; rode de novo com --yes"
    if [ "$KEEP_DATA" = "1" ]; then
        printf 'Continuar? [s/N]: ' > /dev/tty
        read -r ANSWER < /dev/tty || ANSWER=""
        case "$ANSWER" in s|S|sim|Sim|y|Y|yes) ;; *) die "cancelado; nada foi alterado." ;; esac
    else
        printf 'Isto apaga os dados dos serviços. Digite DESTRUIR para continuar: ' > /dev/tty
        read -r ANSWER < /dev/tty || ANSWER=""
        [ "$ANSWER" = "DESTRUIR" ] || die "cancelado; nada foi alterado."
    fi
fi
[ "$DRY" = "1" ] && say "--dry-run: nada será alterado."

# ── 1. Parar o daemon ANTES de tudo ───────────────────────────────────────────
# Por quê primeiro: com o git sync ligado, apagar serviço com o daemon de pé viraria um commit de
# remoção no repositório do sync — exatamente o que a recuperação precisa encontrar intacto.

say "Parando o rustployd e os auxiliares..."
if have_systemd; then
    for u in rustployd.service rustployd-update.socket rustployd-update.service rustployd-fw.socket rustployd-fw.service; do
        run systemctl disable --now "$u"
    done
fi

# ── 2. Regras do ufw (o auxiliar de Podman vem no pacote: tem que rodar antes do purge) ────

if command -v ufw >/dev/null 2>&1; then
    say "Removendo as regras do ufw criadas pelo rustploy..."
    if [ -x /usr/libexec/rustploy-podman-ufw ]; then
        run /usr/libexec/rustploy-podman-ufw remove
    fi
    # Portas liberadas pelo rustployd-fw: `ufw allow <porta>/tcp comment rustploy`. A numeração muda a
    # cada remoção, então consulta de novo a cada volta e apaga a de maior número.
    if [ "$DRY" = "1" ]; then
        ufw status numbered 2>/dev/null | grep -E '# rustploy *$' | sed 's/^/   [dry-run] ufw delete: /' || true
    else
        while :; do
            n="$(ufw status numbered 2>/dev/null | sed -n 's/^\[ *\([0-9][0-9]*\)\].*# rustploy *$/\1/p' | tail -n 1)"
            [ -n "$n" ] || break
            ufw --force delete "$n" >/dev/null 2>&1 || { warn "não consegui apagar a regra $n do ufw"; break; }
        done
    fi
fi

# ── 3. Containers, volumes, redes e imagens ───────────────────────────────────

if [ "$KEEP_DATA" != "1" ]; then
    for e in $ENGINES; do
        say "Removendo os serviços do rustploy no $e..."
        C="$(list_containers "$e")"
        # shellcheck disable=SC2086
        [ -n "$C" ] && run "$e" rm -f -t 5 $C
        N="$(list_networks "$e")"
        # shellcheck disable=SC2086
        [ -n "$N" ] && run "$e" network rm -f $N
        V="$(list_volumes "$e")"
        # shellcheck disable=SC2086
        [ -n "$V" ] && run "$e" volume rm -f $V
        I="$(list_images "$e")"
        # shellcheck disable=SC2086
        [ -n "$I" ] && run "$e" rmi -f $I
    done
else
    say "--keep-data: containers e volumes ficam como estão."
fi

# ── 4. Pacotes ────────────────────────────────────────────────────────────────

if [ -n "$PKGS" ]; then
    say "Removendo os pacotes ($PKGS)..."
    # shellcheck disable=SC2086
    run env DEBIAN_FRONTEND=noninteractive apt-get purge -y $PKGS
    if command -v update-desktop-database >/dev/null 2>&1; then
        run update-desktop-database /usr/share/applications
    fi
fi

# ── 5. systemd e configs que o install.sh gravou fora do pacote ──────────────

say "Removendo drop-ins e configs do instalador..."
run rm -rf /etc/systemd/system/rustployd.service.d
run rm -f /etc/systemd/system/podman.socket.d/rustploy.conf
run rmdir --ignore-fail-on-non-empty /etc/systemd/system/podman.socket.d
run rm -f /etc/containers/registries.conf.d/rustploy.conf /etc/containers/containers.conf.d/rustploy.conf
if have_systemd; then
    run systemctl daemon-reload
    # O drop-in do podman.socket (SocketGroup=rustploy) some: reinicia o socket para valer.
    systemctl is-active --quiet podman.socket 2>/dev/null && run systemctl restart podman.socket
fi

# ── 6. Dados e usuário ───────────────────────────────────────────────────────

if [ "$KEEP_DATA" != "1" ]; then
    say "Apagando os dados do daemon..."
    run rm -rf "$STATE_DIR" "$ETC_DIR"
    if getent passwd rustploy >/dev/null 2>&1; then
        say "Removendo o usuário rustploy..."
        run userdel rustploy
    fi
    if getent group rustploy >/dev/null 2>&1; then
        run groupdel rustploy
    fi
else
    say "--keep-data: $STATE_DIR, $ETC_DIR e o usuário rustploy ficam."
fi

# ── Resto ────────────────────────────────────────────────────────────────────

BACKUPS="$(ls /var/backups/rustploy-* 2>/dev/null || true)"
[ -n "$BACKUPS" ] && warn "sobraram backups com segredos em /var/backups (apague à mão se não precisar):
$BACKUPS"

if [ "$DRY" = "1" ]; then
    say "--dry-run concluído: nada foi alterado."
else
    say "Rustploy removido."
    if [ "$KEEP_DATA" != "1" ]; then
        say "Para trazer tudo de volta: instale de novo (install.sh) e use Settings → Recuperar, com o repositório do sync e a chave de recuperação."
    fi
fi
