# Plano: chave de recuperação gerada e baixada pela interface

> **Status (2026-11-13): IMPLEMENTADO e publicado na v0.1.65** (`glacier-ui` 0.160.0 e `rustploy-shared` 0.29.0 no
> crates.io; daemon 0.1.57 e app 0.1.59). Verificado de verdade no backend: daemon de teste isolado gera a chave, grava só a
> pública no `config.toml`, recifra os segredos, recusa gerar de novo sem `replace`, e a privada baixada abre o segredo
> (`age -d -i`). **Não visto rodando:** a aba no app (o app instalado segura a trava de instância única) e na webui (Chrome
> indisponível); Windows; restauração de um backup S3 com a chave nova.
> Este plano substitui a ideia inicial de um "assistente" de 5 passos: S3 e Git Sync já têm tela própria;
> o único pedaço que faltava era a chave.

## Por quê

Backups e repositório git guardam segredos cifrados com `age`. Sem a chave de recuperação, perder o
`config.toml` do servidor significa perder todos os segredos. Até aqui a chave só nascia pelo terminal
(`rustploy-cli keygen`) e a pública era colada à mão em `[secrets] recovery_recipient`; as abas Backups e
Git Sync apenas avisavam "configure no config.toml".

## O que foi feito

**Protocolo (`rustploy-shared`)** — `RecoveryKeyStatus` → `RecoveryKey { configured, public_key }` e
`RecoveryKeyGenerate { replace }` → `RecoveryKeyGenerated { public_key, private_key, file_contents,
suggested_filename }`.

**Daemon** — `SecretsManager::generate_recovery(replace)` gera o par `age`, grava **só a pública** em
`[secrets] recovery_recipient` (atômico, relendo o config com o parser antes de trocar o arquivo — o
parse é tudo-ou-nada) e devolve a privada **uma única vez**: nem disco, nem memória, nem log.
Recusa se já há chave e `replace` é falso, e se não há `config.toml` onde gravar. Handlers em
`api/handlers/data_backup/recovery_key_*.rs`. `RecoveryKeyGenerate` é **proibido ao agente MCP**
(`agent/tools.rs`); `RecoveryKeyStatus` (só a pública) é admin. Está no `manual.md`.

**webui** — aba Settings → "Chave de recuperação": gera, baixa por `Blob`/`<a download>` (pasta de
downloads do navegador; a webui não sabe o caminho final e diz "baixado pelo navegador"), mostra a privada
uma vez com Copiar / Baixar de novo / "Guardei em lugar seguro". Trocar exige marcar a confirmação. Os avisos
de "sem chave" em Backups e Sincronização ganharam o botão "Gerar chave de recuperação".

**GUI** — mesma aba (`recoverykey.component.gv`, `handlers/recovery_key.luau`), mesmos atalhos nos avisos,
troca com `confirm`. Grava em Downloads por `download.luau` → `save_download`; se não houver pasta
Downloads, cai no diálogo `save_file`.

**glacier-ui** — novo global `save_download(nome, conteúdo)`: pasta Downloads do SO (crate `dirs`),
**sem sobrescrever** (`nome-1.ext`, `nome-2.ext`…), `0600` no Unix, nome sem `../`. Testado em Rust.

**Token do registry Docker** — a mesma regra: ao criar (`RegistryTokenCreate`), o cliente baixa
`rustploy-registry-token-<nome>-<host>-<AAAAMMDD>.txt` (registry, usuário, token e o `docker login`
pronto). Mudança só nos clientes (webui e janela da GUI).

## Decisões

- **A privada nunca fica no servidor.** Quem baixa é sempre o cliente; o daemon roda no servidor.
- **Trocar a chave** não apaga nada: backups e commits antigos continuam abrindo só com a chave antiga.
- **O arquivo é uma cópia em claro** no disco da pessoa (pedido do dono). A tela diz para mover para um
  cofre. Avaliar depois: proteger o arquivo com senha (`age -p`).

## Verificação

Feito: testes do shared (2), do daemon (3 de `generate_recovery`; suíte `api::` e `web` verdes; 1 falha de
ambiente em `external_ports`, porta ocupada), do glacier-ui (2) e `glacier test` da GUI (451 verdes).
**Não feito:** daemon de teste isolado gerando a chave e **restaurando com a privada baixada**; a GUI
rodando de fato (`cargo run`, Xvfb) com o `save_download`; a webui no Chrome; Windows (`dirs` e permissão).
O `cargo` do daemon só compilou com `--config 'patch.crates-io.rustploy-shared.path="../rustploy-shared"'`
(linha de comando, nada commitado), porque o shared novo ainda não está no crates.io.

## Ordem para publicar

1. `glacier-ui`: bump 0.160.0, CHANGELOG, commit, `git fetch` nos dois sentidos, push, `cargo publish`.
2. `rustploy-shared`: bump, commit, push, publish.
3. Subir as versões no daemon e na gui (`cargo update -p …`); GUI usa `glacier-ui = "0.160.0"`.
4. Verificação ponta a ponta acima, `make index`, release e deploy das docs conforme o AGENTS.md.
