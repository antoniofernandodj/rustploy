# Infra-as-Code — Manifestos YAML declarativos

> **Status:** o CLI de arquivo descrito abaixo (`rustploy apply -f`/`rustploy export`,
> `crates/client/src/cli.rs`) vivia no mesmo binário do TUI Ratatui e foi removido junto
> com ele — não há hoje um substituto de linha de comando. `Command::ManifestApply` e
> `Command::ManifestExport` (variante de projeto único) continuam definidos no protocolo,
> mas sem nenhum chamador no repo. O que **continua funcionando** é a tela de Settings do
> `rustploy-gui`, que cobre export/import de **todos** os projetos de uma vez via
> `Command::ManifestExportAll`/`ManifestImport` (YAML + `.env` colados na UI, mesma
> reconciliação aditiva por nome). O restante deste documento descreve o fluxo antigo
> por CLI, mantido como referência histórica do formato do manifesto (que a tela da GUI
> também usa).

Permite descrever projetos e serviços em arquivos `rustploy.yml` versionáveis (GitOps-friendly) e aplicá-los de forma **declarativa e idempotente**, em vez de criar tudo manualmente pela TUI.

O manifesto mapeia quase 1:1 para os modelos internos `Project` e `ServiceSpec`, mas com uma sintaxe ergonômica para edição humana.

> **Onde versionar os manifestos** (repo da API? frontend? infra dedicada?) é tratado em [infra-as-code-organizacao-repo.md](infra-as-code-organizacao-repo.md).

---

## Princípios de design

| Decisão | Comportamento |
|---------|---------------|
| **Escopo** | Dois formatos: um arquivo **por projeto** ou um **manifesto raiz** que agrega vários projetos (inline ou via `include:`). |
| **Reconciliação** | Casa projetos/serviços **por nome**, cria os ausentes e atualiza os alterados. Por padrão **nunca deleta** (aditivo); com `--prune` remove serviços do projeto ausentes no manifesto. |
| **Secrets** | O YAML nunca contém segredo em texto plano. Suporta interpolação `${VAR}` (resolvida no cliente) e referência `secret:NOME` (valor gerenciado fora de banda). |
| **Deploy** | Por padrão `apply` **apenas sincroniza o spec** no banco. Com `--deploy`, dispara o rollout dos serviços criados/alterados após sincronizar. |

---

## Comandos da CLI

Os subcomandos rodam **fora da TUI** (igual ao `import`): leem/escrevem YAML, falam com o daemon e saem.

```bash
# Aplica um manifesto (projeto único ou raiz)
rustploy apply -f rustploy.yml

# Injeta variáveis de um arquivo .env para interpolar ${VAR}
rustploy apply -f rustploy.yml --env-file .env

# Remove serviços do projeto que não constam no manifesto
rustploy apply -f rustploy.yml --prune

# Sincroniza E dispara o deploy dos serviços criados/alterados
rustploy apply -f rustploy.yml --deploy

# Mostra o manifesto resolvido (após interpolação) sem enviar ao daemon
rustploy apply -f rustploy.yml --dry-run

# Exporta o estado atual de um projeto como manifesto (por nome ou id)
rustploy export minha-app
rustploy export minha-app -o rustploy.yml
```

As flags `--prune` e `--deploy` são combináveis (ex.: `apply -f stack.yml --prune --deploy`
converge o estado e faz o rollout numa só chamada — GitOps completo).

Variáveis `${VAR}` não resolvidas (sem valor no ambiente nem no `--env-file`) **abortam** o `apply` com a lista do que faltou — nada é enviado ao daemon.

---

## Formato do manifesto

### Arquivo por projeto

```yaml
apiVersion: rustploy/v1
project:
  name: minha-app
  description: "API e front"
  env:                          # env herdada por todos os serviços
    LOG_LEVEL: info
    DB_PASS: ${DB_PASS}         # interpolado do ambiente/.env no apply
services:
  - name: web
    source:                     # exatamente UMA das três chaves:
      git:                      #   git | registry | compose
        url: https://github.com/acme/web
        branch: main
        dockerfile: Dockerfile
        context: .
    port: 3000
    domain: app.example.com
    tls: true
    env:
      API_TOKEN: secret:api-token   # referência a secret já cadastrado
    volumes:
      - /data/web:/var/lib/web:ro    # host:container[:ro]
    healthcheck:
      type: http                     # none | tcp | http | docker
      path: /health
      status: 200
    replicas: 1
    resources:
      cpu_shares: 512
      mem: 256m                       # aceita sufixos k/m/g
```

### Manifesto raiz (agregador)

```yaml
apiVersion: rustploy/v1
projects:
  - include: ./web/rustploy.yml     # caminho relativo a ESTE arquivo
  - project:                        # ou um projeto inline
      name: infra
      env:
        ENV: prod
    services:
      - name: db
        source:
          registry: postgres:16
        port: 5432
```

O cliente detecta a forma pela chave de topo: `project:` → projeto único; `projects:` → manifesto raiz. Em ambos os casos ele produz uma lista de projetos para enviar ao daemon.

### Tipos de `source`

| Chave | Exemplo | Resultado |
|-------|---------|-----------|
| `registry` | `registry: nginx:1.27` | `ServiceSource::Registry` — pull direto da imagem |
| `git` | bloco com `url`, `branch`, `dockerfile`, `context`, … | `ServiceSource::Git` — clone + build |
| `compose` | `compose: <conteúdo do compose>` | `ServiceSource::Compose` |

Campos do `git` omitidos usam os defaults de `GitSource` (`branch: main`, `dockerfile: Dockerfile`, `context: .`, `root_path: .`).

---

## Como funciona internamente

A peça-chave: a **interpolação `${VAR}` acontece no cliente**, porque depende do ambiente da máquina que roda o `apply`. O **daemon faz a reconciliação**, perto do banco.

```
┌─────────────── cliente (rustploy apply) ───────────────┐
│ 1. lê o arquivo YAML                                    │
│ 2. detecta projeto único vs raiz; resolve `include:`    │
│ 3. interpola ${VAR} (ambiente do processo + --env-file) │
│ 4. re-serializa cada projeto para YAML (String)         │
└───────────────────────────┬─────────────────────────────┘
                            │  Command::ManifestApply { manifests, prune, deploy }
                            ▼
┌─────────────────────────── daemon ──────────────────────┐
│ 5. parseia cada YAML → ProjectManifest (serde_yaml)      │
│ 6. resolve projeto POR NOME → cria ou atualiza           │
│ 7. resolve cada serviço POR NOME → cria ou atualiza      │
│ 8. acumula ResourceAction (created/updated)              │
└───────────────────────────┬──────────────────────────────┘
                            │  Response::ManifestReport(ApplyReport)
                            ▼
            cliente imprime o relatório de ações
```

### Por que os manifestos trafegam como String YAML

O YAML é o formato que o usuário edita, versiona e exporta — mandá-lo cru pelo wire mantém uma só representação de ponta a ponta: o que o daemon recebe é exatamente o texto do arquivo, comentários e formatação inclusive, e o parse acontece num lugar só (`serde_yaml`, do qual o daemon já depende). Reserializar em struct significaria que o texto aplicado e o texto do arquivo poderiam divergir.

Havia também uma razão histórica: o wire era **postcard**, um formato posicional que quebrava com os `skip_serializing_if`/`serde(default)` que os structs do manifesto usam para um YAML enxuto. Esse motivo não existe mais (o wire é JSON, auto-descritivo), mas o design continua valendo pelo primeiro motivo. Tipos "puros" como `ApplyReport` seguem trafegando como struct.

### Interpolação e secrets

Cada valor de `env` é classificado na conversão para `EnvVarValue`:

| Valor no YAML | Vira | Observação |
|---------------|------|------------|
| `KEY: literal` | `Plain("literal")` | config comum |
| `KEY: ${VAR}` | `Plain(<valor resolvido>)` | resolvido no cliente; valor sensível deve vir de `.env` não-versionado |
| `KEY: secret:NOME` | `Secret("NOME")` | referência; valor cadastrado via aba Secrets / `SecretSet` |

> Use `secret:NOME` para segredos de verdade (o valor nunca aparece no YAML nem no banco em texto plano). Use `${VAR}` para config que você não quer commitar mas que não é necessariamente secreta. Veja [secrets.md](secrets.md).

### Reconciliação por nome

- **Projeto**: busca em `db::projects::list` por nome. Igual (description + env) → `Unchanged`. Diferente → `update` + `update_env_vars`. Não existe → `create`.
- **Serviços**: busca em `db::services::list(project_id)` por nome. O spec novo é comparado com o atual (`ServiceSpec` deriva `PartialEq`): idêntico → `Unchanged` (nenhuma escrita); diferente → `update_spec`; ausente → `create`.
- **Sem `--prune`**: serviços no banco que não estão no YAML são **deixados intactos**.
- **Com `--prune`**: esses serviços são removidos (mesma semântica do delete da TUI — remove a rota do ingress e o registro no banco). Projetos **nunca** são removidos por prune.
- **Sem `--deploy`**: nenhum rollout é disparado — só o spec é gravado, pronto para deploy manual.
- **Com `--deploy`**: ao final, cada serviço marcado `Created`/`Updated` tem o deploy disparado via `deploy_start` (serviços `Unchanged` são pulados). As deleções acontecem antes; o deploy, por último.

### Export (round-trip)

`rustploy export` reconstrói o manifesto a partir do estado atual no banco:
- segredos saem como `secret:NOME` (nunca o valor decifrado);
- `mem_limit_bytes` é "humanizado" de volta para `256m`/`2g`;
- defaults de Git/healthcheck são preenchidos explicitamente.

Permite **adotar IaC para projetos que já existem**: exporte, versione o YAML, e passe a usar `apply`.

---

## Arquivos que compõem a funcionalidade

| Arquivo | Papel |
|---------|-------|
| `crates/shared/src/manifest.rs` | **Novo.** Structs do manifesto (`ServerManifest`, `ProjectManifest`, `ServiceManifest`, `SourceManifest`, `HealthcheckManifest`, `ResourcesManifest`), conversões para/de `Project`/`ServiceSpec`, interpolação `${VAR}`, parse de `mem`/volumes, e `ApplyReport`/`ResourceAction`. Inclui testes unitários. |
| `crates/shared/src/lib.rs` | Exporta o módulo `manifest` e seus tipos públicos. |
| `crates/shared/src/protocol.rs` | Adiciona `Command::ManifestApply { manifests, prune, deploy }`, `Command::ManifestExport { project_id }`, `Response::ManifestReport(ApplyReport)` e `Response::Manifest(String)`. |
| `crates/shared/Cargo.toml` | `serde_yaml` como `dev-dependency` (para os testes). |
| `crates/daemon/src/api/handlers/manifest_apply.rs` | **Novo.** Parseia os YAMLs e faz a reconciliação aditiva por nome. |
| `crates/daemon/src/api/handlers/manifest_export.rs` | **Novo.** Monta o `ProjectManifest` a partir do banco e serializa para YAML. |
| `crates/daemon/src/api/handlers/mod.rs` | Registra os dois handlers. |
| `crates/daemon/src/api/routes.rs` | Despacha `ManifestApply`/`ManifestExport` em `dispatch()`. |
| `crates/client/src/cli.rs` | **Novo.** Lógica dos subcomandos `apply`/`export`: leitura de arquivo, `include:`, interpolação, parse de `.env`, envio e impressão do relatório. |
| `crates/client/src/main.rs` | Roteia os argumentos `apply`/`export` para `cli.rs` (antes de entrar na TUI). |
| `crates/client/Cargo.toml` | Adiciona `serde_yaml`. |

> Nenhuma migração de schema: a reconciliação reutiliza integralmente `db::projects` e `db::services`. Como a API HTTP/JSON reusa `Command`/`Response` (via `dispatch`), os subcomandos também funcionam remotamente sem código extra.

---

## Exemplo de fluxo completo

```bash
# 1. aplicar (cria os recursos)
$ DB_PASS=s3cr3t rustploy apply -f rustploy.yml
  [created] project minha-app
  [created] service minha-app/web
🎉 apply concluído: 2 criado(s), 0 atualizado(s), 0 inalterado(s), 0 removido(s).

# 2. aplicar de novo sem mudanças (idempotente — tudo inalterado)
$ DB_PASS=s3cr3t rustploy apply -f rustploy.yml
  [unchanged] project minha-app
  [unchanged] service minha-app/web
🎉 apply concluído: 0 criado(s), 0 atualizado(s), 2 inalterado(s), 0 removido(s).

# 3. GitOps completo: convergir o estado E fazer o rollout numa só chamada
$ DB_PASS=s3cr3t rustploy apply -f rustploy.yml --prune --deploy
  [unchanged] project minha-app
  [updated] service minha-app/web
  [deleted] service minha-app/antigo
🎉 apply concluído: 0 criado(s), 1 atualizado(s), 1 inalterado(s), 1 removido(s).
🚀 deploy disparado para: minha-app/web

# 4. exportar o estado atual (adoção de IaC para o que já existe)
$ rustploy export minha-app -o rustploy.lock.yml
💾 manifesto exportado para rustploy.lock.yml
```

---

## Limitações e próximos passos

- **Prune escopado a serviços**: `--prune` remove apenas serviços ausentes; projetos nunca são removidos automaticamente (use o delete da TUI).
- **`remote-client`**: os subcomandos `apply`/`export` hoje existem no cliente TUI local; replicá-los no `remote-client` é trabalho opcional pendente.
- **Casamento por nome**: renomear um projeto/serviço no YAML cria um novo recurso em vez de renomear o existente (e, com `--prune`, o antigo é removido).
- **Export agregado**: não há `export --all` para gerar um `stack.yml` do servidor inteiro — o export é por projeto.
