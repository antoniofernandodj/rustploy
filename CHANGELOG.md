# Changelog

Histórico das últimas versões do Rustploy (`v0.1.25` a `v0.1.45`, de 28/10/2026 a 07/11/2026), do mais novo para o mais antigo.
Levantado a partir das tags do agregador, dos ponteiros dos submódulos, dos commits entre cada par de tags, das releases do GitHub
e do [`docs/release.md`](docs/release.md) (onde estão as notas de publicação, com o que foi e o que **não** foi verificado).

## Como ler este arquivo

- **A tag não é a versão dos programas.** A tag `vX.Y.Z` é do agregador; o daemon (`rustploy`), o app (`rustploy-gui`) e o
  `rustploy-shared` têm numeração própria, e o `cargo-deb` ainda acrescenta a revisão (`rustployd_0.1.35-1_amd64.deb`). A tabela abaixo
  liga cada tag às versões que ela carrega.
- **Pacotes assinados.** Cada `.deb` e o zip do Windows têm um `.minisig` (minisign, chave do dono). Uma release sem `.minisig` não é
  oferecida pelos updaters. Veja "Assinatura dos pacotes" em [`docs/release.md`](docs/release.md).
- **Release no GitHub × só tag.** Hoje só `v0.1.38` a `v0.1.43` aparecem na lista de releases do GitHub. As anteriores existem como
  **tag** no repositório; o `docs/release.md` registra que várias delas foram publicadas na época (por exemplo `v0.1.31` e `v0.1.35`).
- **`feat!`** marca mudança que quebra compatibilidade (protocolo, YAML ou schema). Quando há `!`, atualize daemon, app e web UI juntos.

## Resumo

| Tag | Data | Daemon (`rustploy`) | App (`rustploy-gui`) | `rustploy-shared` | Destaque |
|---|---|---|---|---|---|
| [v0.1.45](#v0145--2026-11-07) | 2026-11-07 | **0.1.37** | **0.1.44** | **0.20.0** | Edição de variável no lugar, push-to-deploy do registry, `EnvValueSet` |
| [v0.1.44](#v0144--2026-11-06) | 2026-11-06 | **0.1.36** | **0.1.43** | **0.19.0** | Logs por execução: lista, ver, excluir e limpar |
| [v0.1.43](#v0143--2026-11-06) | 2026-11-06 | 0.1.35 | **0.1.42** | 0.18.0 | Seletor de idioma no rodapé do login e em Configurações |
| [v0.1.42](#v0142--2026-11-06) | 2026-11-06 | **0.1.35** | 0.1.41 | **0.18.0** | `product` no lugar de `db_kind` (quebra), webhook form, IP fixo no alias |
| [v0.1.41](#v0141--2026-11-05) | 2026-11-05 | 0.1.34 | **0.1.41** | 0.17.0 | Fix da janela de logs do app |
| [v0.1.40](#v0140--2026-11-05) | 2026-11-05 | **0.1.34** | **0.1.40** | **0.17.0** | Baixar a infra e log de runtime persistido (quebra) |
| [v0.1.39](#v0139--2026-11-05) | 2026-11-05 | **0.1.33** | 0.1.39 | 0.16.0 | Webhook filtra por `watch_paths` |
| [v0.1.38](#v0138--2026-11-04) | 2026-11-04 | **0.1.32** | 0.1.39 | 0.16.0 | Podman + ufw: DNS dos serviços, saída e isolamento |
| [v0.1.37](#v0137--2026-11-03) | 2026-11-03 | 0.1.31 | 0.1.39 | 0.16.0 | Tag a descartar (bug de ordem de regras do ufw) |
| [v0.1.36](#v0136--2026-11-03) | 2026-11-03 | 0.1.30 | 0.1.39 | 0.16.0 | Tag a descartar (manual com receita rejeitada) |
| [v0.1.35](#v0135--2026-11-03) | 2026-11-03 | **0.1.29** | **0.1.39** | **0.16.0** | Credenciais cifradas, renomear variável/secret, Storage |
| [v0.1.34](#v0134--2026-11-03) | 2026-11-03 | 0.1.28 | 0.1.38 | 0.15.0 | Restaurar o servidor a partir do git; `git` no `Depends` |
| [v0.1.33](#v0133--2026-11-02) | 2026-11-02 | 0.1.27 | **0.1.38** | **0.15.0** | Secrets: mover entre escopos, ver valor; dependências na GUI |
| [v0.1.32](#v0132--2026-11-02) | 2026-11-02 | **0.1.22** | **0.1.36** | **0.14.0** | Infra em YAML, secrets age, sync do estado com o git |
| [v0.1.31](#v0131--2026-10-31) | 2026-10-31 | **0.1.20** | **0.1.35** | 0.10.0 | Interface em inglês (padrão) e pt-BR nos dois clientes |
| [v0.1.30](#v0130--2026-10-31) | 2026-10-31 | 0.1.19 | **0.1.33** | 0.10.0 | Rede pelo `http()` do glacier, upload do zip com progresso |
| [v0.1.29](#v0129--2026-10-30) | 2026-10-30 | 0.1.19 | **0.1.32** | 0.10.0 | Abas de Settings estáveis; projeto como link no Deploy Engine |
| [v0.1.28](#v0128--2026-10-29) | 2026-10-29 | 0.1.19 | **0.1.31** | 0.10.0 | Rótulo do link de tabela sem vazar da coluna |
| [v0.1.27](#v0127--2026-10-29) | 2026-10-29 | **0.1.19** | **0.1.30** | 0.10.0 | Validação de variáveis; editor `.env` com erros por linha |
| [v0.1.26](#v0126--2026-10-29) | 2026-10-29 | **0.1.18** | **0.1.29** | 0.10.0 | Links de serviço e projeto nas tabelas |
| [v0.1.25](#v0125--2026-10-28) | 2026-10-28 | **0.1.17** | 0.1.28 | 0.10.0 | Só o bump, para testar o aviso de atualização |
| [v0.1.24](#v0124--2026-10-28) | 2026-10-28 | 0.1.16 | **0.1.28** | 0.10.0 | Aviso de atualização do daemon na tela de login do app |

Em **negrito**, o que mudou em relação à tag anterior. As datas seguem o `docs/release.md` (dia local do dono) e, nas tags sem entrada
lá, a data do commit da tag; o horário de publicação no GitHub, em UTC, está na tabela seguinte.

## Releases publicadas no GitHub (arquivos)

| Release | Publicada (UTC) | Daemon `.deb` (amd64 e arm64) | App `.deb` (amd64) | Windows |
|---|---|---|---|---|
| v0.1.43 | 2026-11-06 07:39 | `rustployd_0.1.35-1` | `rustploy-gui_0.1.42-1` | `rustploy-gui-windows-v0.1.43.zip` |
| v0.1.42 | 2026-11-06 05:50 | `rustployd_0.1.35-1` | `rustploy-gui_0.1.41-1` | `rustploy-gui-windows-v0.1.42.zip` |
| v0.1.41 | 2026-11-06 01:33 | `rustployd_0.1.34-1` | `rustploy-gui_0.1.41-1` | `rustploy-gui-windows-v0.1.41.zip` |
| v0.1.40 | 2026-11-05 07:13 | `rustployd_0.1.34-1` | `rustploy-gui_0.1.40-1` | `rustploy-gui-windows-v0.1.40.zip` |
| v0.1.39 | 2026-11-05 01:14 | `rustployd_0.1.33-1` | `rustploy-gui_0.1.39-1` | `rustploy-gui-windows-v0.1.39.zip` |
| v0.1.38 | 2026-11-04 02:07 | `rustployd_0.1.32-1` | `rustploy-gui_0.1.39-1` | `rustploy-gui-windows-v0.1.38.zip` |

Cada release traz também o `checksums.sha256` e um `.minisig` para cada `.deb` e para o zip.

---

## v0.1.45 — 2026-11-07

Edição de variável no lugar, push-to-deploy do registry embutido e `EnvValueSet`. Muda o daemon (**0.1.37**), o app (**0.1.44**) e o
`rustploy-shared` (**0.20.0**, aditivo). `glacier-ui` **0.151.0** e **0.152.0**.

### Adicionado
- **Editar variável no lugar** (app e web UI, no projeto e no serviço): o ✏️ transforma a chave em um campo na própria linha e o novo ✎ faz o
  mesmo com o valor; Enter grava, ✕ desiste. Antes o ✏️ levava a variável para um formulário no topo da tela, e numa lista grande a janela
  ficava longe dela. Numa variável que aponta para um secret, o campo mostra `secret:NOME`, nunca o valor.
- Protocolo: **`EnvValueSet`** troca só o valor de uma variável, na mesma posição (também disponível ao agente MCP pelo `rpc`).
- **Push-to-deploy do registry embutido**: `ServiceSpec.registry_auto_deploy` (opt-in, na aba Advanced) redeploya o serviço a cada
  `docker push` da mesma imagem e tag; novo `Event::RegistryPush`.
- App: o campo de texto **rola na horizontal** acompanhando o cursor (digitar, colar, setas); o **toast** sobe ao aparecer, tem uma barra de
  tempo lateral e desliza para fora; "Salvar secret" ao lado do valor; a aba Variáveis do projeto rola inteira e não corta mais o botão Adicionar.
- `glacier-ui` 0.152.0: `textinput(autofocus = true)`.

### Alterado
- `/api/rpc` **recusa** (400) um `ServiceUpdate` cujo payload omite campos que o serviço já tem preenchidos (o app e a web UI já devolvem o spec
  completo). O MCP segue só com o aviso do manual.
- Manual do agente: 13 comandos que faltavam (`StateGit*`, `UpdateStatus`/`ApplyUpdate`/`UpdateRollback`, `ServiceWarm*`, `StopContainer`…);
  um teste agora falha se um `Command` novo ficar sem entrada.

### Corrigido
- **Boot:** um serviço com `healthcheck: None` que não escuta porta (um worker de fila) era marcado como erro após 120s em todo boot e arrastava
  quem depende dele. Agora, com o container rodando, conta como pronto.
- A web UI não ficava mais presa ao CSS/JS antigo depois de uma atualização do daemon (o service worker buscava o app shell pelo cache HTTP imutável).
- `parse_mem` estourava o `u64` com um valor enorme num manifesto (`99999999999g`); a fila de deploy sobrevive a um mutex envenenado.

### Publicação
- `rustploy-shared` 0.20.0, `rustploy` 0.1.37 e `rustploy-gui` 0.1.44 no crates.io; release assinada; `glacier-ui` 0.152.0.

---

## v0.1.44 — 2026-11-06

Logs por execução. Muda o daemon (**0.1.36**), o app (**0.1.43**) e o `rustploy-shared` (**0.19.0**, aditivo).

### Adicionado
- Aba **Logs**: "Logs de execução" mostra só a execução atual; abaixo, a lista **Execuções anteriores** (intervalo, deploy, imagem, linhas,
  tamanho, desfecho) com **Ver logs** (janela com data e hora), **Excluir** e **Limpar selecionados**. Na webui e no app.
- Protocolo: `LogsExecutions`, `LogsExecutionDelete` e `container_ids` em `LogsHistory`.
- App: título do serviço editável (grava sozinho), opção nula nos seletores de mover e botões travados sem destino.

### Alterado
- O log de runtime fica em uma pasta por execução; a retenção apaga execuções inteiras. Na primeira subida o daemon migra os logs da v0.1.40.

### Publicação
- `rustploy-shared` 0.19.0, `rustploy` 0.1.36 e `rustploy-gui` 0.1.43 no crates.io; release assinada; `glacier-ui` 0.150.1.

---

## v0.1.43 — 2026-11-06

Seletor de idioma do app. Só o app muda: **`rustploy-gui` 0.1.42**. Daemon 0.1.35 e `rustploy-shared` 0.18.0 não mudam.

### Alterado
- Os botões **English / Português** saíram da barra do topo. Na tela de login ficam em um rodapé discreto, no canto inferior direito;
  dentro do app, numa aba nova **Idioma** em **Configurações**. A web UI não mudou.
- Docs (pt e en) das páginas "Idioma da interface" e "Entrar" atualizadas.

### Publicação
- `rustploy-gui` 0.1.42 no crates.io; release assinada; comando de uma linha do app testado num `debian:12` limpo.

### Verificado
- `cargo fmt`, `clippy -D warnings`, 40 testes; a tela foi aberta e testada pelo dono antes do publish.
- No Chiquitos (daemon 0.1.35 e publisher novo do chickie) os `missed heartbeats` do RabbitMQ sumiram: a `api` ficou 43 min conectada.

### Não verificado
- Reconexão do publisher depois de uma queda do broker em produção (só o teste automático cobre).
- O falso `Stopped` do primeiro deploy depois do restart do daemon (`container not running at read time`) não tem causa conhecida e não repetiu.
- `install.ps1` e a conferência da assinatura num Windows real; atualização do app pelo botão.

---

## v0.1.42 — 2026-11-06

`product` no lugar de `db_kind`, webhook com formulário e IP fixo no alias de rede. **Daemon 0.1.35**, **`rustploy-shared` 0.18.0**;
o app 0.1.41 já usava o `product`.

### Quebra de compatibilidade
- **`ServiceSpec.db_kind` virou `product`** (`rustploy-shared` 0.18.0). O campo guardava banco, broker e storage; o nome mentia. O daemon
  converte o YAML e o SQL antigos na leitura (infra **schema 2**, com backup `infra.bak-schema1`). O manifesto do pacote de serviço
  aceita `db:` como alias de `product:`. O `/api/rpc` recusa `ServiceCreate` e `ServiceUpdate` com `db_kind` preenchido (cliente antigo):
  atualize o app junto.

### Adicionado
- **Webhook aceita `application/x-www-form-urlencoded`** (`payload=`) e ignora o `ping` do GitHub.

### Corrigido
- **IP fixo ao reconectar o alias de rede** (`attach_network_alias`): antes o container voltava com outro IPv4 e as conexões abertas
  (RabbitMQ no Chiquitos) morriam por heartbeat 2–3 min depois do deploy. Se o engine recusar o IP fixo, cai para IP novo com um aviso.
  Validado em Podman rootful 5.7.0, com e sem um segundo container na rede; sem o conserto o teste falha.
- `update_status(Queued|Deploying)` preserva o `live_container_id` e o `reconcile` reconfere o guard de deploy em andamento.
- Template MinIO do catálogo usa `cgr.dev/chainguard/minio`.

### Documentação
- Manifesto do pacote documenta `product` (e `db:` como alias), em pt e en.

### Publicação
- `rustploy-shared` 0.18.0 e `rustploy` 0.1.35 no crates.io. O `rustploy-gui` 0.1.41 já estava publicado.

### Verificado
- `make lint` e `make test` (485 testes do daemon); site com 927 páginas e 0 links quebrados; updater do daemon pelo botão e conversão
  `db_kind` → `product` num servidor com dados reais, sem problemas (conferido pelo dono).

### Não verificado
- IP fixo no Chiquitos foi conferido depois, pelos logs do broker (ver `v0.1.43`).

---

## v0.1.41 — 2026-11-05

Fix da janela de logs do app. Só o app: **`rustploy-gui` 0.1.41**. Daemon 0.1.34 e shared 0.17.0 inalterados.

### Corrigido
- A janela de logs da v0.1.40 dava `attempt to call a nil value` ao abrir **Logs**: `format.history_rows` não estava exportado pela fachada
  `views/scripts/format.luau`.
- O separador de container do log passa a sair em pt-BR no catálogo pt-BR (aparecia em inglês).

### Verificado
- `cargo fmt`, `clippy -D warnings`, 40 testes e `cargo publish --dry-run`; a GUI abre a janela de logs da `api` do Chiquitos e mostra o
  histórico (11 linhas).

### Não verificado
- O botão **Carregar anteriores** (só aparece com mais de 500 linhas guardadas num serviço que não seja Compose).

### Armadilha
- Função nova em `format/*.luau` precisa entrar também na fachada `views/scripts/format.luau`; os testes Luau não pegam a falta.

---

## v0.1.40 — 2026-11-05

Baixar a infra e log de runtime persistido. **Daemon 0.1.34**, **app 0.1.40**, **`rustploy-shared` 0.17.0** (quebra de protocolo).

### Quebra de compatibilidade
- Saem a aba **Infra as Code** e os comandos `ManifestApply`, `ManifestExport`, `ManifestExportAll` e `ManifestImport`. GUI e web UI antigas
  contra este daemon perdem a aba IaC: atualize os dois juntos.

### Adicionado
- **Baixar a infra** (`InfraDownload`): devolve os arquivos que o `state_git` grava, com os secrets como `ENC[age:…]`; recusa com
  `PlaintextInInfra` se o git também recusaria. Botão na aba **Sync** da GUI e da web UI.
- **Log de runtime persistido** (`LogsHistory`): arquivos por serviço em `<dados>/logs/`, 14 dias e 50 MiB (`[logs]` no `config.toml`),
  capturados antes de remover o container (troca e rollback). A janela de logs semeia do histórico e ganha **Carregar anteriores**.
- Abrir **Settings** carrega a lista de contas Git sozinho (GUI e web UI).

### Corrigido
- Links de serviço e projeto voltam a clarear no hover.

### Alterado
- O daemon deixa de referenciar o `rustploy-shared` por `path`: usa o 0.17.0 do crates.io. `cargo install rustploy` volta a refletir a `main`.

### Publicação
- `rustploy-shared` 0.17.0, `rustploy` 0.1.34 e `rustploy-gui` 0.1.40 no crates.io.

### Verificado
- `make lint` e `make test` (476 testes do daemon), `glacier test` 399, web UI headless 16, site 927 páginas e 0 links quebrados, CI da tag,
  comando de uma linha num `debian:12` limpo (daemon e app, com a assinatura conferida), `InfraDownload` e o log persistido num daemon local.

### Não verificado
- A captura do container antigo na troca (`Promoting`) num engine real; a janela de logs da GUI nativa; o `install.ps1`.

### Armadilhas
- O Podman entrega timestamps de log com resolução de segundo: a deduplicação do `log_store` precisa de instantes únicos.
- `pkill -f` casa com a própria linha de comando do shell: mate por PID exato.

---

## v0.1.39 — 2026-11-05

Webhook filtra por `watch_paths`. **Daemon 0.1.33**; app 0.1.39 (formatação dos `.gv`); shared 0.16.0 não mudou.

### Adicionado
- O webhook de deploy (`POST /webhook/<id>/<token>`) respeita o `watch_paths` do serviço Git: com o campo preenchido, só dispara se algum
  arquivo do push (`commits[].added|modified|removed`) casar; senão responde `200 skipped: no watched path changed`.
- **Na dúvida faz o deploy** (nunca perde um push): corpo sem a lista, `commits` vazio, commit sem alguma das listas, merge sem arquivos ou
  10+ commits. Deploy manual e o do MCP ignoram o filtro. Aceita arquivo, diretório ou glob simples (`*`, `**`).
- Para que serve: várias aplicações na mesma branch (chickie deixa as branches `deploy/*`/`main-*` e o `make sync`).

### Documentação
- Manual do agente: bucket e migrations como passo de provisionamento em pré-deploy; o comando de gerar/rotacionar segredo ainda não existe.

### Verificado
- `make lint` e `make test` (462 testes do daemon, 10 do filtro) e um daemon local real com 8 payloads. Release conferida em `debian:12`
  (daemon 0.1.33 e app 0.1.39).

### Não verificado
- Com um GitHub de verdade (o formato do payload vem da documentação dos provedores); `install.ps1`; o daemon sob systemd.

---

## v0.1.38 — 2026-11-04

Podman + ufw: DNS dos serviços e saída dos containers. **Daemon 0.1.32**; app 0.1.39; shared 0.16.0.

### Corrigido
- Em servidor com Podman rootful e `ufw` (`deny incoming` e `deny routed`), os nomes `rp_*` não resolviam dentro dos containers e nada saía
  para a internet. Novo auxiliar `rustploy-podman-ufw` (em `/usr/libexec/`), chamado pelo `postinst` e pelo `install.sh --engine podman`:
  detecta as faixas das redes, libera DNS e saída e **isola as redes entre si** (o `deny` antes do `allow`).
- O auxiliar confere a **ordem** das regras de rota e refaz com o `deny` no topo (`ufw route insert 1`): com o `allow` já presente, um `deny`
  acrescentado depois ficava atrás e perdia o efeito.
- `ServiceReload` não relê secrets (só para e inicia o mesmo container): trocar um secret exige redeploy; o manual diz isso.

### Verificado
- Num servidor real (DNS por nome, saída e portas internas com o isolamento ativo) e num laboratório
  (`tests/e2e-podman-ufw/run.sh`: Podman rootless privilegiado, `ufw` 0.36 e `iptables-nft` reais).

### Observação
- As tags `v0.1.36` e `v0.1.37` ficaram no repositório porque apagar tag remota foi bloqueado. Vale a `v0.1.38`.

---

## v0.1.37 — 2026-11-03

**Não usar.** Daemon 0.1.31. Tirou do manual a "receita" de rotação de senha por serviço descartável, mas o auxiliar do ufw tinha um bug de
ordem de regras, achado depois pelo laboratório e corrigido na `v0.1.38`. A execução foi cancelada; descarte qualquer rascunho que apareça.

---

## v0.1.36 — 2026-11-03

**Não usar.** Daemon 0.1.30. Liberou o ufw para os containers do Podman (DNS por nome e saída, com detecção das faixas) e trouxe no manual uma
receita de rotação de senha por serviço descartável, **rejeitada pelo dono**. A execução foi cancelada; **não assine o rascunho**.

### Adicionado
- `feat(podman)`: libera o ufw para os containers; receita de bucket com `curl --aws-sigv4` (o `aws-cli` rejeita hostname com `_`).

---

## v0.1.35 — 2026-11-03

Credenciais cifradas por padrão, renomear variável/secret, Storage (RustFS e MinIO) e agente que nunca vê segredo. **Daemon 0.1.29**,
**`rustploy-shared` 0.16.0**, app 0.1.39.

### Adicionado
- **Identidade `age` no lugar da `master.key`** para cifrar os secrets.
- **Renomear** chave de variável (`EnvKeyRename`) e secret (`SecretRename`); senhas do wizard geradas pelo daemon.
- **Storage** no wizard (RustFS e MinIO) com credenciais geradas e cifradas; categoria `storage` na tela Em execução.
- **O agente (MCP) nunca recebe o valor de um secret** (Fase A).
- Web UI: instruções de permissões do token GitHub (PAT fine-grained). Campo de senha só onde existe (Kafka e NATS não têm).

### Corrigido
- Padrões de usuário e banco do wizard quando o chamador não preenche; o ✏️ da variável do serviço na linha normal, não na de arrastar;
  MinIO usa `cgr.dev/chainguard/minio`; `clippy::double_must_use` do `async_trait` (clippy 1.99).

### Observação
- Nesta release o daemon referenciava o `shared` por `path` e o `cargo install rustploy` entregava a 0.1.28. Resolvido na `v0.1.40`.
- `minio/minio` e `minio/mc` não são mais puxáveis (Docker Hub e Quay).

### Verificado
- `make lint`, `make test` por repo, `glacier test` (406), e2e do updater (11 cenários), site com 0 links quebrados; comando de uma linha
  pela URL pública em `debian:12` (daemon 0.1.29 e app 0.1.39); `install.ps1` até o SHA-256 num PowerShell no Linux.

---

## v0.1.34 — 2026-11-03

Restaurar o servidor a partir do git. **Daemon 0.1.28**; app 0.1.38; shared 0.15.0.

### Adicionado
- **Restaurar num servidor limpo a partir do git** (Fase 8 do sync de estado).
- **`git` no `Depends` do `.deb`** (Fase 9): o clone de serviços Git usa o `git` do servidor.

---

## v0.1.33 — 2026-11-02

Secrets entre escopos e dependências na GUI. **Daemons 0.1.23 a 0.1.27**, **app 0.1.38**, **`rustploy-shared` 0.15.0**.

### Adicionado
- **Secrets do serviço** na aba **Environment**: listar, salvar, apagar e mover variável para secret (GUI e web UI).
- **Mover variável entre serviço e projeto** (`SecretMoveOp::EnvServiceToProject` e `EnvProjectToService`), com 🔒 🔓 ↑ ↓ na GUI.
- **Ver o valor de um secret** (👁 🙈 📋), do projeto e do serviço, na GUI e na web UI.
- **Dependências e religar no boot** na aba **Advanced** do serviço (web UI e GUI).

### Alterado
- O daemon **deixa de criar a `master.key`**; só a lê se já existir.

### Corrigido
- `rustployd-fw.service` sobe em servidor sem `ufw` (`ReadWritePaths` com `-`); o log mostra a causa completa quando a identidade não cabe
  no `config.toml`.

---

## v0.1.32 — 2026-11-02

Infra em YAML como fonte da verdade, secrets `age`, dependências e sincronização do estado com o git. **Daemon 0.1.22**,
**app 0.1.36**, **`rustploy-shared` 0.14.0**.

### Adicionado
- **Infra em YAML como fonte da verdade**, com secrets `age`, dependências entre serviços e autostart (`ServiceSpec.depends_on`, shared 0.11.0).
- **Secrets com escopo de serviço**, `SecretMove`, `SecretReveal` e `InfraAudit` (shared 0.12.0).
- **Sincronização do estado com o git** (`StateGit*`, `InfraAuditAllow`; shared 0.13.0), com a Fase 7: fetch e pull validado, pausa por
  divergência, e itens removidos só marcam. Tela **Settings → Sync**, aviso no topo e containers órfãos com Parar/Remover (GUI e web UI).

### Alterado
- Templates `.gvb` viram `.gv` e o vocabulário passa a snake_case; `glacier-ui` 0.149.0.

---

## v0.1.31 — 2026-10-31

Interface em inglês (padrão) e pt-BR nos dois clientes. **App 0.1.35**, **daemon 0.1.20**; shared 0.10.0.

### Adicionado
- **Seletor de idioma** (English e Português) lembrado entre sessões, no app desktop e na web UI. Textos de tela passam a ser comparados em inglês nos testes.

### Alterado
- `width` e `spacing` dos `form(...)` vão para classes do `.gss`; `text("…")` sem `content =` em todo o código; o hover da aba ativa
  clareia o azul em vez de cair no cinza (app 0.1.34).

### Verificado
- Comando de uma linha pela URL pública em `debian:12`: app (`rustploy-gui 0.1.35-1`) e daemon (`rustployd 0.1.20-1`; sem systemd no
  contêiner, o serviço não sobe). A conferência da assinatura no Windows segue sem teste real.

### Armadilha
- `make test` caiu no link da GUI (`signal 7 [Bus error]`) com o disco a 92% enquanto rodavam outros builds: não é do código.

---

## v0.1.30 — 2026-10-31

Rede e upload do app. **`rustploy-gui` 0.1.33**; daemon 0.1.19; shared 0.10.0.

### Adicionado
- Rede pelo `http()` do `glacier-ui` 0.145; upload do zip com **progresso e Cancelar**; **atualizar o app em thread, sem `curl`**; reconexão
  do SSE; hover com `transition`.
- Projeto como link na fila e em "Executando" do Deploy Engine.

### Alterado
- Gradiente leve nos botões `primary`, `blue` e `danger` (`glacier-ui` 0.142).

---

## v0.1.29 — 2026-10-30

Abas de Settings estáveis. **`rustploy-gui` 0.1.32**; daemon 0.1.19; shared 0.10.0.

### Corrigido
- As abas de Settings não oscilam mais (`tabbar active_bold=false`, `glacier-ui` 0.140).

### Alterado
- O projeto vira link no histórico do Deploy Engine.

---

## v0.1.28 — 2026-10-29

**`rustploy-gui` 0.1.31**; daemon 0.1.19; shared 0.10.0.

### Corrigido
- O rótulo do `TableLink` usa 13px para não vazar da coluna.

---

## v0.1.27 — 2026-10-29

Validação de variáveis e editor `.env`. **Daemon 0.1.19**, **`rustploy-gui` 0.1.30**; shared 0.10.0.

### Adicionado
- **Validação de variáveis de ambiente** (nome e chave repetida) com aviso dos erros do editor `.env`.
- Editor `.env` da GUI com **erros por linha**, nome inválido e confirmação ao sobrescrever.
- Serviços do Deploy Engine viram links; as colunas truncam antes de vazar.

---

## v0.1.26 — 2026-10-29

Links nas tabelas. **Daemon 0.1.18**, **`rustploy-gui` 0.1.29**; shared 0.10.0.

### Adicionado
- Nomes de serviço e projeto viram links em Monitoring, Deployments e Ingress (web UI e GUI); selos sem colar e "Serviços no ar" no cabeçalho.

---

## v0.1.25 — 2026-10-28

**Daemon 0.1.17**; app 0.1.28; shared 0.10.0.

### Alterado
- Só o bump de versão, para testar o aviso de atualização na tela de login do app.

---

## v0.1.24 — 2026-10-28

**`rustploy-gui` 0.1.28**; daemon 0.1.16; shared 0.10.0.

### Adicionado
- Aviso de atualização do daemon na tela de login do app.

---

## Pendências conhecidas (até a v0.1.43)

- `install.ps1`: o atalho do Menu Iniciar (COM do Windows) e a assinatura com o `minisign.exe` (Windows PowerShell 5.1) não têm teste real.
- O `.deb` arm64 só foi compilado, nunca instalado numa máquina arm64.
- A migração Docker → Podman com dados reais foi testada só com stubs.
- O falso `Stopped` ("container not running at read time") visto uma vez no primeiro deploy depois do restart do daemon não tem causa
  conhecida; se repetir, o `journalctl -u rustployd` do momento é o que falta.
- Detalhes e armadilhas de cada publicação: [`docs/release.md`](docs/release.md).
