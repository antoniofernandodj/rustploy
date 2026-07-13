# 2026-07-13 — Remoção do TUI + Registry Docker embutido (Fase 2)

Resumo das duas frentes de trabalho fechadas nesta data, na ordem em que foram pedidas.

---

## 1. Registry Docker embutido — Fase 2 (auth + exposição pública)

Contexto completo, decisões de design e arquivos tocados estão em
[`docs/plano-registry-embutido.md`](plano-registry-embutido.md) (nota datada de
2026-07-13 logo no topo). Resumo do que foi entregue:

- **Basic auth obrigatória em TODA rota do registry** (`crates/daemon/src/registry/auth.rs`),
  inclusive `GET /v2/` e inclusive em loopback — não há bypass por origem, porque o
  listener em `127.0.0.1:5100` é alcançável por qualquer processo do host.
  - Escopos `Pull`/`Push`; um token `push` também satisfaz `pull`.
  - Sem credencial ou credencial inválida → `401` + `WWW-Authenticate: Basic
    realm="rustploy-registry"` (`RegistryError::Unauthorized`).
- **Tokens de acesso**: tabela `registry_tokens` (`crates/daemon/src/db/registry_tokens.rs`)
  — só o hash SHA-256 do segredo é persistido; o segredo em texto plano só existe na
  resposta de criação (`Command::RegistryTokenCreate` → `Response::RegistryTokenCreated`),
  nunca mais depois disso. `RegistryTokenList`/`RegistryTokenRevoke` completam o CRUD.
- **Exposição pública opcional**: campo `daemon_settings.registry_domain` (mesma
  precedência banco > config já usada pelo e-mail do ACME), com `ingress.upsert_route` +
  `tls.ensure_cert` disparados tanto no boot quanto ao mudar em runtime
  (`set_daemon_settings.rs`). Sem domínio configurado, o registry continua acessível só
  via loopback (uso local, sem TLS).
- **GUI**: campo de domínio em Settings → Web Server; seção "Tokens de acesso" na sub-aba
  Registry (criar/listar/revogar); janela dedicada `new_registry_token_window.xml` que
  mostra o comando `docker login` pronto para copiar (segredo visível só naquela janela,
  na criação).
- **Validado com smoke test real** (não só testes automatizados): `docker login` +
  `push`/`pull` autenticados, 401 sem credencial, token `pull` corretamente barrado de
  fazer push, revogação derrubando acesso na hora, domínio configurado registrando rota
  real no ingress (confirmado via `curl`).
- **Não incluído nesta fase**: token interno `rp-internal` para o deploy executor
  (fica para a Fase 3 — só faz sentido quando o executor for de fato consumi-lo).

Os 4 testes de integração HTTP pré-existentes (que assumiam ausência de auth) foram
ajustados para autenticar; testes novos cobrem os casos de 401/escopo insuficiente.

---

## 2. Remoção do TUI (`crates/client`)

### O que foi removido

- A crate inteira `crates/client/` (binário `rustploy`, Ratatui) — `git rm -rf`.
- Do workspace: `Cargo.toml` (`members`), `.github/workflows/release.yml` (step "Build
  client" e empacotamento do binário `rustploy`), `measure_ram.sh` (script de comparação
  de RAM daemon vs. TUI, sem mais sentido sem o segundo processo).
- Alvos do `Makefile` que só existiam para o TUI: `dev-tui`, `install-tui`, a variável
  `CLIENT_BIN`, e a linha do target `install` que instalava `rustploy_*.deb`.

### Achado importante: o TUI carregava também o CLI de Infra-as-Code

`crates/client` não era só o TUI — o mesmo binário também implementava os subcomandos
`rustploy apply -f`/`rustploy export` (`crates/client/src/cli.rs`), a parte de
linha-de-comando da feature de Infra-as-Code (manifestos YAML declarativos, ver
[`docs/infra-as-code.md`](infra-as-code.md)). Isso não apareceu nas referências óbvias a
"TUI"/"Ratatui" — só foi encontrado ao varrer o repo por `crates/client` antes de
comitar.

Depois de confirmado o achado, a decisão (explícita, pelo usuário) foi **aceitar a perda
do CLI de arquivo por enquanto**, em vez de extraí-lo para uma crate própria. Consequências:

- `rustploy apply -f rustploy.yml` / `rustploy export` **não existem mais** como comando
  de linha de comando. Não há substituto de CLI hoje.
- **A feature em si não foi perdida por completo**: a tela de Settings do `rustploy-gui`
  já cobre export/import de **todos** os projetos via `Command::ManifestExportAll` /
  `Command::ManifestImport` (YAML + `.env` colados na UI, mesma reconciliação aditiva por
  nome) — isso é uma feature *diferente e mais recente* que o CLI por arquivo/projeto
  único, mas cobre o caso de uso central (declarar e reconciliar via YAML).
- `Command::ManifestApply` e `Command::ManifestExport` (a variante de **projeto único**,
  distinta de `ManifestExportAll`) continuam definidos em `crates/shared/src/protocol.rs`
  e roteados em `api/routes.rs`, mas **sem nenhum chamador no repo** — ficaram órfãos.
  Não foram removidos nesta rodada (mexer em variantes de enum do protocolo tem custo/risco
  à parte, e não foi pedido); se um dia a limpeza completa for feita, os handlers
  `manifest_apply.rs`/`manifest_export.rs` também precisam sair.
- `docs/infra-as-code.md` ganhou uma nota de status no topo explicando exatamente essa
  distinção (CLI morto vs. GUI viva) — o resto do documento (formato do manifesto,
  princípios de reconciliação) continua válido porque é compartilhado pelas duas vias.

### Documentação atualizada

- **`CLAUDE.md`**: seção de build simplificada (sem comando de rodar o TUI), tabela de
  crates sem a linha `client`, seção "IPC protocol" reescrita — `rustploy-gui` é hoje o
  único cliente (HTTP/JSON+SSE); o listener UDS/postcard do daemon continua existindo no
  código mas sem nenhum consumidor no repo (mexer nele só se for pedido explicitamente).
- **`README.md`**: tabela comparativa, seção de cliente (singular), build/execução e
  arquitetura atualizados; nota de rodapé no changelog de fases marcando o TUI como
  concluído-e-depois-removido; bullet de Infra-as-Code reescrito para refletir a via GUI.
- **`AGENTS.md`** (spec técnica antiga, ~830 linhas): as duas seções inteiras dedicadas ao
  TUI (`## 8. Crate client — TUI` e `## 16. Use Cases do Aplicativo TUI`, juntas ~370
  linhas) foram removidas; as ~15 menções soltas nas outras seções foram ajustadas
  (tabela comparativa, árvore de diretórios, roteiro de implementação, dependências) —
  ou generalizadas para "cliente" quando o mecanismo (EventBus, eventos `LogLine`)
  continua válido para o `rustploy-gui`, ou marcadas explicitamente como histórico
  removido. A nota de desatualização já existente no topo da seção 17 (que já avisava
  sobre SurrealDB→SQLite) foi expandida para cobrir também a remoção do TUI e a migração
  KDL→XML/Luau, e aponta pro `CLAUDE.md` como referência atual.
- **`GEMINI.md`** (50 linhas, estava descrevendo uma arquitetura de duas gerações atrás —
  `client` como TUI, templates em `crates/client/src/templates/`): reescrito por completo
  para refletir a arquitetura atual (crates reais, embedded registry, Luau, blueprints em
  `crates/shared/templates/`).
- **`docs/memoria-threads-e-runtime.md`**: única referência factual quebrada
  (`measure_ram.sh`, que citava o binário do TUI) corrigida; o resto do documento (teoria
  de VSZ/RSS/threads) é conteúdo atemporal, não mexido.
- **`docs/plano-dependencias-e-autostart.md`** (plano futuro, ainda não implementado):
  ganhou uma nota avisando que os passos que tocavam `crates/client/src/models.rs`/
  `events.rs` (aba Advanced do TUI) estão obsoletos — quando essa feature for
  implementada, a UI correspondente vai para o Luau do `rustploy-gui`, não para arquivos
  Rust do TUI que não existem mais.
- **`docs/relatorio-porta-externa-automatica.md`**: não alterado — é um relatório de uma
  feature já concluída, a menção ao TUI ali é registro histórico correto para a época,
  não uma afirmação sobre o estado atual do projeto.

### Verificação

`cargo check --workspace` limpo depois de todas as mudanças (a remoção da crate e os
ajustes de documentação não quebraram nada). Nenhuma referência funcional a
`crates/client`/`-p client` sobrou em CI, packaging ou `Makefile`.
