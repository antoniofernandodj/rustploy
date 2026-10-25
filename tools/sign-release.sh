#!/usr/bin/env bash
# Assina os pacotes de uma release em rascunho e a publica.
#
# O release.yml cria a release como RASCUNHO e não assina nada: a chave privada nunca vai
# para o CI (docs/plano-updaters-daemon-e-gui.md, seção 3). Este script roda na máquina do
# dono: baixa os pacotes do rascunho, assina cada um com `minisign` (senha da chave pedida
# a cada arquivo), confere a assinatura com a chave pública, sobe os `.minisig` e publica.
#
# Uso:  tools/sign-release.sh vX.Y.Z [--key CAMINHO] [--no-publish] [--dry-run]
#
# A chave padrão é ~/.minisign/rustploy.key (a pública fica ao lado, rustploy.pub).
# --no-publish  assina e sobe os .minisig, mas deixa a release em rascunho.
# --dry-run     baixa e assina, mas não sobe nada nem publica.

set -euo pipefail

die() { echo "erro: $*" >&2; exit 1; }
say() { echo "==> $*"; }

TAG=""
KEY="$HOME/.minisign/rustploy.key"
PUBLISH=1
DRY=0

while [ $# -gt 0 ]; do
    case "$1" in
        --key) [ $# -ge 2 ] || die "--key pede um caminho"; KEY="$2"; shift 2 ;;
        --no-publish) PUBLISH=0; shift ;;
        --dry-run) DRY=1; shift ;;
        -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
        v*) TAG="$1"; shift ;;
        *) die "argumento desconhecido: $1 (uso: $0 vX.Y.Z [--key CAMINHO] [--no-publish] [--dry-run])" ;;
    esac
done

[ -n "$TAG" ] || die "informe a tag (ex.: v0.2.0)"
VERSION="${TAG#v}"
PUB="${KEY%.key}.pub"

for tool in minisign gh dpkg-deb sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || die "falta o comando '$tool'"
done
[ -f "$KEY" ] || die "chave privada não encontrada: $KEY (gere com: minisign -G -p ${PUB} -s ${KEY})"
[ -f "$PUB" ] || die "chave pública não encontrada: $PUB"

# Só mexe em rascunho: uma release já publicada não deve ganhar assinatura depois, e
# um rascunho errado é melhor descoberto antes de qualquer upload.
IS_DRAFT="$(gh release view "$TAG" --json isDraft --jq .isDraft 2>/dev/null || true)"
[ -n "$IS_DRAFT" ] || die "release $TAG não encontrada (o CI já terminou o job 'release'?)"
[ "$IS_DRAFT" = "true" ] || die "a release $TAG já está publicada; este script só assina rascunhos"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

say "baixando os pacotes de $TAG"
gh release download "$TAG" --dir "$WORK" --pattern '*.deb' --pattern '*.zip' --pattern 'checksums.sha256'

shopt -s nullglob
PKGS=("$WORK"/*.deb "$WORK"/*.zip)
[ ${#PKGS[@]} -gt 0 ] || die "o rascunho $TAG não tem .deb nem .zip"

# O checksums.sha256 do CI tem de bater com o que baixamos: pega um rascunho adulterado
# ou incompleto antes de assinar (assinar é o ato que dá confiança ao arquivo).
if [ -f "$WORK/checksums.sha256" ]; then
    say "conferindo checksums.sha256"
    (cd "$WORK" && sha256sum -c checksums.sha256) || die "o checksums.sha256 não bate com os arquivos baixados"
else
    die "o rascunho não tem checksums.sha256"
fi

for f in "${PKGS[@]}"; do
    name="$(basename "$f")"
    case "$name" in
        *.deb)
            pkg="$(dpkg-deb -f "$f" Package)"
            ver="$(dpkg-deb -f "$f" Version)"
            arch="$(dpkg-deb -f "$f" Architecture)"
            # Sem comparar com a tag: a tag do agregador (v0.1.16) e a versão do pacote
            # (0.1.10-1, crate + revisão do cargo-deb) são numerações diferentes. O que
            # vale para o updater é a versão do próprio pacote, que vai no comentário assinado.
            ;;
        rustploy-gui-windows-*.zip)
            pkg="rustploy-gui-windows"; ver="$VERSION"; arch="amd64"
            ;;
        *) die "arquivo inesperado no rascunho: $name" ;;
    esac
    # O comentário confiável entra na assinatura: o updater compara estes campos com os do
    # próprio pacote (nome, versão, arquitetura) antes de instalar.
    comment="package=$pkg version=$ver arch=$arch"
    say "assinando $name ($comment)"
    minisign -S -s "$KEY" -t "$comment" -m "$f"
    minisign -V -p "$PUB" -m "$f" >/dev/null || die "a assinatura de $name não confere com $PUB"
done

SIGS=("$WORK"/*.minisig)
if [ "$DRY" = 1 ]; then
    say "--dry-run: ${#SIGS[@]} assinatura(s) gerada(s), nada foi enviado"
    exit 0
fi

say "enviando ${#SIGS[@]} arquivo(s) .minisig para $TAG"
gh release upload "$TAG" "${SIGS[@]}" --clobber

if [ "$PUBLISH" = 1 ]; then
    say "publicando $TAG"
    gh release edit "$TAG" --draft=false
    say "pronto: https://github.com/$(gh repo view --json nameWithOwner --jq .nameWithOwner)/releases/tag/$TAG"
else
    say "assinado; a release segue em rascunho (--no-publish). Publique com: gh release edit $TAG --draft=false"
fi
