# Plano: YAML como fonte da verdade da infra, com git e religamento em ordem

> ## ⚠ LEIA PRIMEIRO: este documento tem duas camadas
> - **Seções R1–R9 (a partir de "Revisão 2026-11-01") são o plano VIGENTE.** Foram decididas com o
>   usuário em 2026-11-01 e prevalecem sobre tudo o que está acima delas.
> - **Seções 1–10 são o plano ORIGINAL (2026-10-26):** backup como *espelho* de um SQLite que seguia
>   dono do estado. Ficam como histórico do raciocínio (regra "comentar, não remover"). Cada seção
>   antiga traz, logo abaixo do título, uma nota `SUBSTITUÍDO` ou `AINDA VALE` dizendo o que fazer com ela.
> - **Em caso de dúvida, vale a R.** Não implemente nada só pelas seções 1–10.
>
> Mapa rápido do que mudou: fonte da verdade (SQLite → YAML, R2/R3), segredos (passphrase
> `master.key` + `secrets.age` por projeto → identidade `age` no `config.toml` + `ENC[age:…]`
> inline, R5), fases (§8 → R9), restauração (§7 → R6/R8/R9), mais três assuntos novos: escopo de
> secrets, GitOps completo e dependências/autostart.

**Estado:** **planejado, nada implementado.** Escrito em 2026-10-26; revisado em 2026-11-01 (ver R1–R9).
**Sessão de chat:** a conversa que projetou este plano pode ser retomada com `claude --resume 9f61d38c-7929-4171-a58e-17c9f2fe6fb9`.

---

## 1. O problema, em linguagem simples

> **AINDA VALE em essência** (perder o servidor perde tudo), mas a solução mudou: ver R1.

Hoje todo o estado declarado do servidor (projetos, serviços, variáveis de ambiente, segredos,
provedores git, bancos gerenciados) mora em **um arquivo SQLite no disco do próprio servidor**.
Se o servidor se perde (disco morre, VPS apagada), perde-se tudo junto.

O que já existe protege contra acidente, **não** contra perda do servidor:

- `features/backup` grava snapshots JSON das env vars a cada 60 s, mas **no mesmo disco**
  (`<db_path>/env_backups/`). Ver [env-backup.md](env-backup.md).
- `ManifestExportAll` exporta tudo num `rustploy.yml` + `rustploy.vars.toml`, mas **só quando
  alguém clica** na tela de Settings.

O objetivo: **a cada mudança, o daemon grava o estado num repositório git privado no GitHub**,
de modo que dê para (a) ver o histórico de quem mudou o quê e (b) reconstruir um servidor
novo a partir dele, **inclusive os segredos**.

## 2. A decisão de arquitetura (e o que ficou de fora)

> **SUBSTITUÍDO por R1–R3.** O SQLite deixa de ser dono da infra; o YAML passa a ser.

**O SQLite continua sendo a fonte da verdade.** O git é um **espelho de mão única**: só o
daemon escreve nele; ninguém edita o YAML à mão.

Por que não o contrário (YAML como fonte, SQLite só para runtime)? Foi analisado e descartado
nesta sessão: a GUI e a webui teriam de reescrever o arquivo a cada clique (comentários e
ordem se perdem, working tree sujo, dois escritores no mesmo arquivo), parte do estado nasce
em runtime (portas, tokens de webhook), e a troca mexe na camada `store` inteira. O ganho
(git como fonte) não é o que se quer aqui; o que se quer é **versionar e recuperar**.
Com espelho de mão única somem de graça os problemas de drift, de `managed_by: git` e de
conflito de merge.

Também descartado: um YAML único gigante (conflito, um erro derruba tudo, diff ilegível) e
versionar o `.db` em git (binário, sem diff, sem merge).

**Um YAML por projeto**, mais um arquivo raiz que os agrega, é o formato (o manifesto atual
já suporta `include:`).

## 3. O que já existe e será reaproveitado

> **PARCIALMENTE VÁLIDO.** `ManifestExportAll`/`ManifestImport` **não** são mais a base (R9.0 item 3: formato interno próprio; o manifesto fica como está). `SecretsManager` muda de passphrase para identidade `age` (R5). `features/backup` (env a cada 60 s) é **aposentado na Fase 0** (R9.0 item 4).

| Peça | Onde | Para quê |
|---|---|---|
| `ManifestExportAll` | `features/iac/manifest_export_all.rs` | gera o manifesto raiz; já redige env `Plain` para `${KEY}` e separa os valores reais num TOML (`EnvDoc`) |
| `ManifestImport` | `features/iac/manifest_import.rs` | restaura, reconciliando por nome |
| `SecretsManager` | `platform/secrets.rs` | cifra `age` (passphrase) em hex; chave em `/etc/rustploy/master.key` |
| `features/backup` | `features/backup/mod.rs` | tarefa periódica com `interval`; modelo para o novo gatilho |
| `git_provider` / `webhook_token` | `features/git_providers/`, `store/` | infraestrutura de credenciais git |

## 4. Segredos: o ponto crítico

> **PARCIALMENTE SUBSTITUÍDO por R5.** A tabela 4.1 (onde cada dado sensível está hoje) e a regra 4.2 ("nada vai ao git sem passar por `age`") **continuam valendo**. O 4.3 (dois cofres, `secrets.age` por projeto) e a passphrase **estão superados**.

### 4.1 Onde cada dado sensível está hoje (verificado no código)

| Dado | Como está guardado | Situação |
|---|---|---|
| `secret.value` (segredos de projeto) | cifrado (`age`, hex) | pode ir ao git como está |
| `git_provider`: `oauth_client_secret_enc`, `access_token_enc`, `refresh_token_enc` | cifrado | idem |
| `registry_tokens.token_sha256` | hash | idem (mas o token real não é recuperável) |
| **env `Plain` dos serviços** (`service.spec`, JSON) | **texto puro** | o export atual os tira do YAML e põe no `rustploy.vars.toml` **em claro** |
| **`managed_database.password`** | **texto puro** | o export atual **nem inclui** bancos gerenciados |
| **`webhook_token.token`** | **texto puro** | não é exportado |
| `master.key` | arquivo em `/etc/rustploy/` | o próprio segredo mestre |

Consequência: o `rustploy.vars.toml` que o export já produz **não pode ir ao git do jeito que
está**, porque carrega valores reais em claro. E os bancos gerenciados precisam entrar no
backup, senão uma restauração perde as senhas dos bancos.

### 4.2 A regra

> **Nada vai ao git sem passar por `age`.** O exportador trata todo campo como sensível por
> padrão e deixa em claro **só uma lista explícita de campos seguros** (nome, imagem, porta,
> domínio, replicas…). Campo novo adicionado no futuro nunca vaza por esquecimento.

### 4.3 Dois cofres, duas chaves

> **SUBSTITUÍDO por R5.** Agora: identidade `age` X25519 no `/etc/rustploy/config.toml`, valores inline `ENC[age:…]` no YAML, vários destinatários (servidor + chave pública de recuperação opcional). Não implemente `secrets.age` por projeto.

- **No repo:** todo conteúdo sensível vai num arquivo `*.secrets.age` por projeto, cifrado
  para uma **chave pública de recuperação** (`age` com *recipient*, não passphrase).
- **A chave privada correspondente** fica **fora do servidor e fora do git** (gerenciador de
  senhas, cofre offline). Só você a tem.
- A `master.key` de produção **não precisa sair do servidor**: o daemon decifra com ela, e
  recifra para a chave pública de recuperação na hora de exportar.

Assim, vazar o repo não revela nada, perder o servidor não perde nada, e a chave
de recuperação é **um item pequeno que você guarda uma vez**.

Alternativa mais simples (pior): commitar o ciphertext que já está no banco e guardar a
`master.key` à parte. Funciona, mas obriga a levar a `master.key` do servidor (que muda de
dono/máquina) e amarra o backup à rotação dela.

### 4.4 Cuidados já identificados

> **AINDA VALE** (`master.key`/config em 0600 vira Fase 0; repo sempre privado). A frase sobre rotação da `master.key` está superada por R5.

- `SecretsManager::new` cria a `master.key` com `std::fs::write` (permissão padrão, em geral
  0644). Restringir a **0600** na criação e corrigir as já existentes. Independe deste plano,
  mas ele aumenta a importância do arquivo.
- O token/deploy key para dar `push` é mais um segredo: guardá-lo cifrado como os demais.
- Repositório **privado** sempre: mesmo sem segredos, o YAML expõe topologia, domínios e imagens.
- **Rotação da `master.key`** recifra o banco, mas o repo está cifrado para a chave de
  recuperação, não para a `master.key`, então a rotação não invalida o histórico. É uma
  vantagem da §4.3.

## 5. Formato do repositório

> **SUBSTITUÍDO por R3 ("Formato dos arquivos") e R9.0 item 3.** Nada de `secrets.age` nem de reaproveitar o formato do manifesto. **Continuam valendo:** serialização determinística, ids preservados, nada de runtime no YAML.

```
<repo>/
├── stack.yml                  # raiz: lista e `include:` dos projetos
├── projects/
│   ├── <projeto>/
│   │   ├── rustploy.yml       # serviços, domínios, jobs (sem valores sensíveis)
│   │   └── secrets.age        # envs Plain + secrets + senhas de bancos (cifrado)
├── providers.yml              # git providers (sem tokens)
├── providers.secrets.age      # tokens/segredos OAuth
└── MANIFEST.json              # versão do daemon, versão do schema, data, hash do estado
```

Regras do conteúdo:

- **Serialização determinística**: projetos e serviços ordenados por nome, chaves em ordem
  estável (`BTreeMap` já é usado no export). Sem isso, todo export gera diff de ruído.
- O `id` de projeto/serviço entra no YAML. É isso que preserva a identidade na restauração
  (o histórico de deploys aponta para IDs, não para nomes).
- Nada de runtime: `deployment`, `build_log`, `job_run`, `registry_*`, `tls_cert` ficam de fora.
- **`secrets.age` precisa ser estável**: `age` gera salt/nonce novos a cada cifragem, então
  recifrar o mesmo conteúdo produz bytes diferentes e um commit a cada ciclo. Solução: só
  recifrar quando o **conteúdo em claro** mudou (comparar o hash do plaintext, guardado em
  `MANIFEST.json`, nunca o ciphertext).

## 6. Gatilho e fluxo

> **SUBSTITUÍDO por R6** (modos `off`/`commit`/`sync`, auditoria antes de ligar, fetch por intervalo + botão, GitOps completo).

1. Algo declarativo muda (criar/editar/apagar projeto, serviço, segredo, provider, banco).
   O daemon já tem um barramento de eventos em `platform/`; publicar um evento
   `StateChanged` ali (ou observar os eventos existentes).
2. **Debounce** de alguns segundos (várias edições seguidas viram um commit só).
3. Gerar o export em um diretório temporário, comparar com o checkout local do repo.
   **Sem diff, sem commit.**
4. Commit com mensagem útil (`service api: env alterado`) e `push`.
5. Falha de push (rede, credencial) **não atrapalha o daemon**: loga, mantém o commit local
   e tenta de novo no próximo ciclo. O estado nunca bloqueia por causa do backup.

Configuração em `config.toml` (parse é tudo-ou-nada; campo novo precisa de default):

```toml
[state_git]
enabled = false                 # desligado por padrão
remote  = "git@github.com:org/rustploy-state.git"
branch  = "main"
debounce_secs = 10
recovery_recipient = "age1..."  # chave PÚBLICA de recuperação
```

## 7. Restauração

> **SUBSTITUÍDO por R6 (aplicar árvore), R8 (religamento em ordem) e R9 Fase 8.** O ponto "o `ManifestImport` gera IDs novos" continua verdadeiro, e por isso existe a função própria de aplicar árvore.

1. Instalar o rustploy no servidor novo.
2. Clonar o repo, fornecer a **chave privada de recuperação**.
3. `rustploy restore <repo>` (ou botão em Settings): decifra `*.secrets.age`, importa o YAML
   com os IDs preservados, grava os segredos **recifrando com a `master.key` nova**.
4. Serviços voltam como `Stopped`; um redeploy (ou o autostart, quando existir) os sobe.

O `ManifestImport` atual reconcilia por nome e **gera IDs novos**. Preservar os IDs é um
trabalho novo (modo `restore` separado do `apply`, para não mudar o comportamento atual).

## 8. Fases

> **SUBSTITUÍDO por R9.** A lista de fases abaixo (0–5) é a antiga; a vigente tem as fases 0–9.

| Fase | Entrega | Risco |
|---|---|---|
| **0** | Fechar o que falta saber (ver §10) e restringir a `master.key` a 0600 | baixo |
| **1** | Exportador determinístico por projeto + `*.secrets.age` (sem git ainda): `Command::StateExport` que grava num diretório. Teste: exporta duas vezes, diff vazio | médio |
| **2** | Restauração com IDs preservados (`StateRestore`) e teste de ida e volta: exporta, apaga o banco, restaura, compara | médio-alto |
| **3** | Gatilho com debounce + commit/push (usar `git` do sistema ou `git2`; **sem reqwest**, ver memória) + `[state_git]` na config | médio |
| **4** | UI: Settings na GUI **e** na webui (estado, último commit, botão "exportar agora", botão "restaurar"), conforme a regra dos dois clientes | baixo |
| **5** | Docs (`rustploy-docs`, pt e en) e checklist de "Publish" | baixo |

A ordem importa: **sem a Fase 2 provada, o backup é uma promessa não testada.** Só considerar
a feature pronta depois de restaurar de verdade num servidor limpo.

## 9. Testes

> **PARCIALMENTE VÁLIDO.** Determinismo, vazamento, ida e volta, chave errada e push com falha continuam; os testes por fase estão em R9.1.

- Determinismo: dois exports seguidos do mesmo estado geram bytes idênticos (menos `*.age`,
  que se compara pelo plaintext).
- **Vazamento**: teste que percorre o diretório exportado e falha se qualquer valor de env,
  senha ou token conhecido do banco de teste aparecer em claro em qualquer arquivo.
- Ida e volta: exporta, apaga o banco, restaura, e o `ManifestExportAll` do resultado é igual
  ao original.
- Chave errada: restaurar com a chave de recuperação errada falha sem escrever nada.
- Push com falha não derruba nem bloqueia o daemon.
- Por ser feature de UI também: rodar a GUI e a webui de verdade antes de dar por pronto.

## 10. Pontos em aberto

> **Em grande parte RESOLVIDO** pela revisão: `service.env` e `managed_database.password` em claro (confirmado, tratado em R5); `daemon_settings` (varrido, R2.1); `git` do sistema (decidido, R9.0). Os demais pontos migraram para a R7/R8.6.

- ~~`service.env` e `managed_database.password` estão em claro?~~ **Confirmado:** ambos em texto
  puro (env no JSON de `service.spec`; a senha vai direto do banco para a view em
  `databases/managed_view.rs`).
- `git` do sistema ou crate `git2`/`gix`? O `git` do sistema é mais simples, mas vira dependência
  de runtime do pacote `.deb`. Decidir na Fase 3.
- **Mais algum segredo não passa pelo cofre?** Falta varrer `daemon_settings` (pode guardar
  credenciais), as credenciais de ACME e o token da API, que está no `config.toml`, não no banco.
  O token da API e a `master.key` ficam fora deste backup (são do servidor, não do estado); a
  restauração gera os novos.
- Os blobs do registry embutido e os volumes dos containers **não** estão no escopo: este
  plano cobre configuração, não dados. Backup de dados (bancos, volumes) é outro assunto.
- Estratégia de retenção: o histórico git cresce a cada mudança. Provavelmente aceitável (texto
  pequeno), mas vale medir antes de prometer.

---

# Revisão 2026-11-01 — YAML como fonte da verdade da infraestrutura (RASCUNHO EM DISCUSSÃO)

**Estado:** em conversa com o usuário; nada implementado. Esta revisão **muda o plano**: as seções
1–10 acima tratavam o YAML como *espelho* de backup (SQLite continuava dono do estado). Aqui o
**YAML passa a ser o dono dos dados de infraestrutura**, e o SQLite fica só com o runtime.
Onde esta revisão contradiz as seções 1–10, vale esta. Vamos preenchendo conforme a conversa.

## R1. Decisões já tomadas

1. Dados de **infraestrutura** moram em YAML; dados de **runtime** continuam em SQL.
2. Uma **camada de repositório** é o único caminho de escrita da infra. Ninguém mexe nos arquivos direto.
3. Toda edição é uma **transação: tudo ou nada**, com lock e troca atômica (ver R3).
4. Na atualização que introduz isto, o daemon **migra os dados de infra do SQL para o YAML** uma vez.
5. Senhas de bancos hoje vivem em env/texto puro: devem ir para **secrets cifrados** (ver R5).
6. A **sincronização automática com git** (commit/pull/push) tem liga/desliga e **só pode ser
   ligada quando não sobra nenhum valor sensível em claro** (ver R6).
7. A **chave de criptografia** vive em `/etc/rustploy/config.toml`.
8. Mover env↔secret é **ação manual na GUI**, nos dois sentidos (R5). Bancos gerenciados **novos** nascem com a senha em secret.
9. Migração do SQL é **uma vez, na atualização**; depois o SQL guarda só runtime (R4).
10. Pull não fast-forward: o daemon **para a sincronização e avisa**, nunca tenta resolver sozinho (R6).
11. Secrets ganham escopo (projeto ou serviço). Ações manuais enxutas (tabela em R5): mover entre env/secret/escopo e "ver valor" com auditoria. Ninguém usa secrets hoje, então sem migração entre escopos nem diálogos de conflito.
12. **GitOps completo** (o `pull` aplica no daemon em execução), com guarda-corpos (R6) e um CLI próprio (`rustploy-cli secret encrypt`) para escrever segredos à mão.
13. Job com serviço gatilho enxerga secrets do gatilho e do projeto; secret inexistente vira erro de deploy (R5).
14. `SecretReveal` **fora do MCP/`rpc`** (lista de comandos proibidos); só GUI e webui.

## R2. O que é infra (YAML) e o que é runtime (SQL)

Regra prática: **se um humano declarou, é infra; se o sistema produziu, é runtime.**

| Tabela atual | Destino | Observação |
|---|---|---|
| `project` | **YAML** | inclui env de projeto |
| `service` | **YAML + SQL** | `spec` (JSON) vai para YAML; `status` e `live_container_id` são runtime e ficam no SQL (tabela nova `service_runtime`) |
| `managed_database` | **YAML** | senha vira secret (R5) |
| `secret` | **YAML** (valores cifrados inline) | ganha escopo: projeto ou serviço (R5) |
| `git_provider` | **YAML + SQL** | config e `client_secret` cifrado no YAML; `access/refresh token` no SQL (runtime) |
| `job` | **YAML** | definição e agendamento |
| `shared_access`, `webhook_token` | **YAML** | `webhook_token` é segredo → cifrar |
| `daemon_settings` | **a varrer** | pode ter credenciais; decidir por chave |
| `deployment`, `build_log`, `job_run`, `job_log` | **SQL** | runtime |
| `registry_*`, `registry_tokens`, `tls_cert` | **SQL** | runtime/estado de terceiros |
| `migration` | **SQL** | controle de schema |

Ponto de atenção: `service` mistura as duas naturezas na mesma linha. Separar `status` do `spec`
é a parte mais invasiva do plano (todo código que lê `service.status` passa a ler outra fonte).

### R2.1 Varredura feita em 2026-11-01: `daemon_settings` e `ServiceSpec`

**`daemon_settings`** (tabela chave→valor que a tela de Settings edita). Nenhuma chave guarda
credencial (o que preocupava o §10):

| Chave | Natureza | Destino |
|---|---|---|
| `acme_email` | configuração | YAML (`settings.yml`) |
| `registry_domain` | configuração | YAML |
| `docker_cleanup_config` | configuração (JSON) | YAML |
| `docker_cleanup_last_run` | resultado da última execução | SQL |
| `build_cache_last_prune` | resultado da última limpeza | SQL |
| `job_warm:<job_id>`, `service_warm:<service_id>` | marcas de aquecimento | SQL |
| `webhook_base_url` | **deprecada**, ninguém lê nem escreve | descartar (não migrar) |

Regra: a tabela continua para o runtime; as três chaves de configuração migram para o YAML e a
tela de Settings passa a escrever pelo repositório. As chaves que mudam toda hora
(`*_last_run`, `warm`) nunca tocam o git.

**`ServiceSpec`** é, no geral, declarativo. A varredura achou **quatro** pontos que misturam
coisa gerada pelo sistema:

1. **`host_port`** — o usuário pode pedir porta automática (`Some(0)`); o daemon escolhe uma
   livre (`external_ports::resolve_host_port`) e **grava a escolhida no spec**. É infra (o
   firewall depende dela), mas gerada. Regra: vai ao YAML já com o número alocado, e a alocação
   roda *dentro* da transação. No GitOps, um commit externo com `host_port: 0` é aceito: o daemon
   aloca e grava o número de volta (um commit seguinte). Ao restaurar, revalidar a faixa, pois
   dois servidores podem disputar a mesma porta.
2. **`compose_project`** — nome da stack Compose, gravado na criação e nunca recalculado (os
   volumes se chamam `<stack>_<volume>`). Hoje está numa coluna fora do `spec`. **Tem de ir ao
   YAML**, senão restaurar renomeia a stack e os volumes "somem".
3. **`ArchiveSource.archive_id`** — aponta para um zip no disco do daemon, não no banco. O YAML
   leva o id, **não o arquivo**; num servidor novo o id não aponta para nada. Regra: o serviço
   restaura, mas o deploy falha com "arquivo ausente, envie de novo". Backup dos zips é fora de escopo.
4. **`created_at`/`updated_at`** — `updated_at` muda a cada edição e só geraria diff. `created_at`
   vai ao YAML; `updated_at` fica fora.

Há código que reescreve o `spec` sem o usuário pedir (`databases/env_switch.rs`, o restaurador
de env em `features/backup`, a alocação de porta). Tudo passa pela camada de repositório como
qualquer escrita e, com o git ligado, gera commit normalmente.

## R3. A camada de repositório e a transação

### Por que o daemon, e só ele, é dono dos arquivos
O daemon é um processo único. Logo, **leituras vêm de memória** (um `Arc<InfraState>` imutável atrás
de um `RwLock`); o disco é só persistência. Isso resolve, de graça, o medo de "alguém ler no meio
da edição": o daemon nunca lê arquivo parcial, porque lê a cópia em memória. O único leitor
externo é o `git`, e para ele a regra é: só enxerga o diretório quando a transação já terminou.

### Fluxo de uma escrita
1. Pega o **lock de escrita** (um `Mutex` em memória + `flock` num arquivo `infra/.lock`, para
   proteger contra duas instâncias do daemon por engano).
2. Copia o estado atual, aplica a mudança em memória e **valida tudo** (referências entre
   entidades, nomes únicos, schema). Falhou? Descarta, nada foi escrito.
3. Serializa **de forma determinística** (ordem estável) e grava **só os arquivos que mudaram** em
   `infra/.staging/<txid>/`, com `fsync` em cada um.
4. Grava o marcador `infra/.staging/<txid>/COMMIT` (com `fsync` do diretório). **Este é o ponto de
   não retorno.**
5. Move (`rename`, atômico por arquivo) cada arquivo do staging para o lugar final e apaga o staging.
6. Publica o novo `Arc<InfraState>` em memória e solta o lock.

### Recuperação de queda (o que torna "tudo ou nada" real)
`rename` de **um** arquivo é atômico; de **vários**, não. Então a atomicidade vem do marcador:
- Caiu **antes do COMMIT** (passo 4): na subida, o staging é apagado. Estado antigo intacto.
- Caiu **depois do COMMIT**: na subida, o daemon **termina de aplicar** o staging (roll-forward).
Resultado: depois de qualquer queda, ou a transação inteira valeu ou nenhuma parte valeu.

### Alternativa considerada: trocar o diretório inteiro
Gravar um diretório novo e trocar por inteiro (`renameat2 RENAME_EXCHANGE`, ou um symlink
`current -> v123`) é atômico de verdade e dispensa a recuperação. **Descartada como padrão** porque
o git precisa de um diretório de trabalho estável (`.git` dentro dele) e a troca de diretório
quebraria isso. Fica como plano B se o roll-forward der trabalho.

### Formato dos arquivos
Um YAML por projeto (`projects/<nome>/project.yml`), mais `providers.yml` e `settings.yml`.
Arquivos pequenos = diffs pequenos, menos conflito no git, e uma transação que toca um projeto só
reescreve um arquivo. `MANIFEST.json` guarda a versão do schema (para migrações futuras do próprio YAML).

## R4. Migração na atualização (SQL → YAML)

Na primeira subida da versão nova, se `infra/` não existe e o SQL tem dados de infra:
1. Faz **cópia de segurança do SQLite** (`<db>.pre-yaml.bak`).
2. Lê as tabelas de infra e escreve o YAML **por uma única transação** do repositório.
3. Faz a **verificação de ida e volta**: relê o YAML e compara com o que veio do SQL. Divergiu →
   aborta, remove `infra/`, o daemon continua rodando na versão antiga dos dados.
4. Marca a migração como concluída (linha em `migration`).
5. As tabelas antigas **não são apagadas** (regra "comentar, não remover"): ficam como rollback
   por pelo menos uma versão.

Interpretação da frase "recuperar todos os dados do sql": **migrar uma vez, na atualização, e
daí em diante o SQL só guarda runtime.** **Confirmado pelo usuário em 2026-11-01.**

## R5. Criptografia e secrets

### O que muda em relação à seção 4
A §4.3 propunha uma chave de recuperação fora do servidor. O usuário quer a chave **em
`/etc/rustploy/config.toml`**. Proposta que une as duas:

- Trocar a passphrase (`age` scrypt, lenta e sem destinatários) por uma **identidade `age`
  X25519**: `age_identity = "AGE-SECRET-KEY-…"` no `config.toml`. O daemon deriva a chave pública.
- O YAML guarda cada valor sensível **inline**, no formato `ENC[age:<base64>]`. Vantagens sobre um
  `secrets.age` por projeto: o diff do git mostra *qual* chave mudou (sem mostrar o valor), e a
  estrutura do YAML continua legível.
- **Vários destinatários**: a chave do servidor + uma `recovery_recipient` (pública) opcional. Só
  assim dá para restaurar num servidor novo sem ter a chave antiga.
- Arquivo `config.toml` com permissão **0600** (hoje a `master.key` nasce com a padrão). O parse
  é tudo-ou-nada: campo novo com default, e erro claro se a identidade for inválida.
- **Risco a aceitar de olhos abertos:** perder o `config.toml` sem ter a chave de recuperação =
  perder todos os segredos. Por isso a UI deve avisar, ao ligar o git, se `recovery_recipient`
  estiver vazio.

### Migração dos segredos existentes
Os `secret.value` atuais estão cifrados com a passphrase da `master.key`. Na migração (R4) o
daemon decifra com a `master.key` antiga e recifra com a identidade nova. A `master.key` antiga
fica até a versão seguinte, só como rollback.

### Mover envs para secrets (e de volta): ação manual na GUI

**Decisão do usuário (2026-11-01): nada migra sozinho.** Nem as senhas de bancos, nem as envs
"suspeitas". O daemon não adivinha nem move por conta própria; quem decide é a pessoa, env por
env, e a mudança vale nos **dois sentidos**.

O modelo já comporta isso: `EnvVarValue` tem as variantes `Plain(String)` e `Secret(String)`, e
`Secret` guarda só o **nome** do secret (a resolução em `env_resolve` troca pelo valor na hora do
deploy). Então mover é uma operação pequena, feita numa única transação do repositório (R3):

- **Env → secret** ("Mover para secret"): cria o secret `<nome>` com o valor atual (cifrado) e
  troca a env de `Plain(valor)` para `Secret(nome)`. Se já existir um secret com esse nome, a operação
  **falha** com mensagem clara (regra simples da próxima subseção).
- **Secret → env** ("Mover para env"): decifra o valor e troca a env de `Secret(nome)` para
  `Plain(valor)`. O secret é apagado (mover = criar no destino e apagar na origem). Avisa antes: *com a sincronização git ligada, isso põe o valor em claro no YAML e a
  auditoria vai bloquear/desligar o git* (R6).
- Vale para env de **serviço** e de **projeto**. Fica na tela de variáveis, nos **dois clientes**
  (GUI iced e webui), com botão travado até a resposta (`busy_actions`).
- Comando novo no protocolo (ex.: `EnvMoveToSecret` / `SecretMoveToEnv`), sempre atômico: ou o
  secret e a env mudam juntos, ou nada muda. Nunca fica env apontando para secret inexistente.
- Nada precisa ser redeployado só por mover: o valor resolvido é o mesmo. (Verificar se alguma
  parte do daemon compara o `spec` para decidir "mudou, precisa redeploy" e se a troca
  `Plain`→`Secret` dispara isso à toa.)

### Secrets com escopo: de serviço e de projeto (simplificado em 2026-11-01)

**Pressuposto do usuário: ninguém usa secrets hoje.** Logo, não há o que preservar nem migrar
entre escopos; se a tabela `secret` tiver linhas na hora da migração (R4), elas viram secrets de
projeto, recifrados, e pronto. Nada de diálogo de conflito, "copiar" ou análise de quem usa.

**Modelo.** Um secret pertence ao **projeto** ou a **um serviço** (hoje só existe o de projeto: a
tabela `secret` tem só `project_id`, e `env_resolve` sempre chama `get_raw(project_id, nome)`).
Nome único dentro do escopo. No YAML, o de serviço fica no bloco do serviço e o de projeto no do
projeto, sempre `ENC[age:…]`. **Resolução** de `Secret(nome)` numa env de serviço: escopo do
serviço primeiro, depois o do projeto. Env de projeto só enxerga o de projeto. **Job com serviço gatilho** enxerga os dois (serviço gatilho primeiro, depois projeto); job autônomo, sem gatilho, só o de projeto.

**As ações são exatamente estas, cada uma um botão, tudo manual:**

| Onde está | Operações |
|---|---|
| **Secret de serviço** | → env do serviço · → secret de projeto · ver valor |
| **Secret de projeto** | → secret de **um serviço específico** (a GUI pergunta qual) · → env de projeto · ver valor |
| **Env de serviço** | → secret do serviço |
| **Env de projeto** | → secret do projeto |

Regras, deliberadamente simples:
- **Mover = criar no destino e apagar na origem, numa transação só** (R3). Se der erro, nada muda.
- **Se já existe no destino um item com o mesmo nome, a operação falha** com mensagem clara. O
  usuário apaga ou renomeia à mão e tenta de novo. Sem sobrescrever, sem diálogo de escolha.
- **Secret → env:** as envs do mesmo escopo que apontavam para o secret (`Secret(nome)`) viram
  `Plain(valor)`; se nenhuma apontava, cria-se uma env com chave = nome do secret. O secret some.
- **Env → secret:** cria o secret com o valor e troca a env para `Secret(nome)`.
- **Não analisa quem mais usa o nome.** Se outro serviço ainda apontava para um secret de projeto
  que foi movido, esse deploy falha com erro claro (ver abaixo), não em silêncio. A GUI pode
  listar as referências como aviso, mas não bloqueia.
- **Secret que não existe vira erro de deploy (decidido).** Hoje `unwrap_or_default()` devolve string vazia
  em silêncio; passa a falhar o deploy dizendo qual secret falta e onde foi procurado. É o que
  torna seguro o "não analisa quem usa".
- Cada ação é um comando próprio no protocolo, com botão travado até a resposta (`busy_actions`),
  nos **dois clientes** (GUI iced e webui).

**O caso de uso:** cria-se um banco; a senha nasce como secret **do serviço do banco**; o usuário
clica em "→ secret de projeto" e qualquer outro serviço do projeto passa a referenciar
`Secret("DB_<nome>_PASSWORD")`. Para fechar o acesso, "→ secret de um serviço".

### Botão "ver valor" (revelar)

- Comando novo `SecretReveal`, **separado** do `SecretList` (que segue devolvendo só nomes). O
  valor nunca vem em lista, snapshot do dashboard ou log (o `tracing` redige o campo).
- Na UI: valor **mascarado**; o botão revela e **esconde de novo sozinho** (~15 s) e ao sair da
  tela. Não fica guardado no estado da UI além disso.
- Cada revelação grava uma **linha de auditoria** (quem, qual secret, quando; nunca o valor).
- **Decidido: fora do MCP.** A ferramenta `rpc` do MCP aceita "qualquer Command", então precisa de
  uma **lista de comandos proibidos** na camada MCP, com `SecretReveal` nela, e a ferramenta `rpc`
  devolve erro explicando. O objetivo é o agente **não ler segredo por engano**.
- **Limite honesto:** isso protege contra leitura acidental, não contra um agente determinado.
  O token da API é o mesmo da GUI, então quem tiver o token e fizer HTTP direto ainda alcança o
  comando. Se isso importar no futuro, o caminho é separar o *tipo* de credencial (a chave do
  agente não ganha permissão de revelar), conversa que já tem terreno em
  `docs/plano-chave-de-acesso-e-mcp.md`.

**Bancos gerenciados.** A senha de `managed_database.password` é um caso à parte: ela não é uma
env que o usuário digitou, o daemon a gera e a injeta. Como a regra agora é "tudo manual",
proponho que a senha do banco **nasça já como secret** em bancos novos (decisão de design, não
migração) e que os existentes **sigam em claro até o usuário mover** pelo mesmo botão. **Decidido (2026-11-01): exceção aceita** — bancos novos já nascem com as credenciais em secret **de serviço**; os
existentes só mudam por ação manual.

**Consequência para o git:** como nada migra sozinho, a auditoria da R6 é o que impede o
vazamento. Enquanto sobrar env/senha em claro, a sincronização com git **não liga**, e a UI diz
exatamente quais são (a lista que serve de guia para o usuário ir movendo). O gravador do YAML
continua tratando todo campo como sensível por padrão, com lista explícita de campos seguros (§4.2).

## R6. Liga/desliga da sincronização com git

Três modos, em `config.toml` (`[state_git]`), alteráveis pela UI:

| Modo | Faz |
|---|---|
| `off` (padrão) | YAML só no disco local |
| `commit` | commita localmente a cada transação (com debounce); não fala com a rede |
| `sync` | commit + pull + push |

**Trava para ligar:** passar de `off` para `commit`/`sync` só é permitido se a **auditoria**
passar: nenhum valor de env, senha ou token em claro no YAML. Se falhar, a UI mostra o que falta
(a lista de envs/senhas em claro) e recusa. Como o git só liga *depois* do conteúdo estar limpo, **o histórico
nunca contém um segredo em claro** — o que é crucial, pois apagar segredo de histórico git não
funciona de verdade.

### O que o `pull` faz — duas opções, em detalhe

Primeiro, a ideia comum: o daemon mantém um clone local do repo no próprio diretório `infra/`.
`commit` e `push` saem do estado do daemon para o git. A pergunta é só o sentido contrário.

**Opção A — `pull` só para restaurar e sincronizar máquinas (mais simples).**
- O daemon em execução **nunca** aplica nada que veio do git. O git é um *destino* (backup que
  conta a história), e o YAML local continua sendo o que manda.
- O `pull` só roda em dois momentos: ao **restaurar** num servidor novo (clone + importar) e em
  uma ação explícita "sincronizar agora", que traz o remoto *se o servidor local estiver limpo*.
- Para saber se o remoto andou por fora, o daemon faz `git fetch` periódico (só olha, não aplica).
  Se o remoto tem commits que o local não tem, aplica a regra de conflito (parar e avisar).
- Quem edita o YAML direto no git (ex.: pelo GitHub) **não** vê o efeito no servidor, e o `push`
  seguinte vai recusar por não ser fast-forward. Aceitável: a edição deve ser feita pela GUI.

**Opção B — GitOps de verdade: edição por fora no git é aplicada no daemon em execução.**
Fluxo: `fetch` periódico → há commit novo e o local não tem nada pendente → `fast-forward` →
compara a árvore antiga com a nova → **valida tudo** → aplica como **uma transação** do
repositório (R3). O mesmo código de "carregar uma árvore, validar e substituir o estado" serve à
migração (R4), à restauração e a este caminho; por isso vale escrevê-lo uma vez, bem testado.

O que B exige, e que não é óbvio (agora é requisito, não opção):
1. **YAML inválido não pode derrubar nada.** Se a validação falhar, o daemon **não aplica**,
   mantém o estado atual, marca a sincronização como *bloqueada* e mostra o erro com o SHA do
   commit. O remoto fica à frente até alguém corrigir.
2. **Valor em claro vindo de fora é recusado.** A auditoria da R6 vale também para o que chega:
   um commit externo que ponha senha em claro é rejeitado (senão o `pull` furaria a regra
   "nada em claro no git"). Para escrever um segredo à mão, a pessoa precisa de uma ferramenta
   que cifre com a chave pública (`rustploy secret encrypt`), e isso é trabalho extra.
3. **Sem laço.** Aplicar o que veio do git muda o estado, o que dispararia um commit e um
   `push`. O estado aplicado já *é* o `HEAD`, então não pode haver commit novo (compara-se com o
   diff real, nunca com "mudou algo").
4. **O que acontece com o que está rodando?** Mudou o `spec` de um serviço no git: o daemon só
   atualiza o *declarado* e marca "mudança pendente de deploy" (indicador de divergência). Não
   redeploya sozinho. Auto-deploy ao receber é uma opção por serviço, para uma fase futura.
5. **Remoção é perigosa.** Sumir um serviço do YAML não pode apagar container e dados na hora.
   Padrão: o serviço fica marcado "removido no git" e a remoção real exige confirmação na GUI
   (ou uma flag de *prune* desligada por padrão).
6. **Edições da GUI em paralelo** são serializadas pelo mesmo lock de transação; se a GUI
   escreveu enquanto o `fetch` rodava, vira divergência e aplica a regra de conflito.
7. **Gatilho (decidido):** `fetch` por **intervalo fixo** (`[state_git] fetch_interval_secs`) **mais um botão "sincronizar agora"** na GUI e na webui. Sem webhook por enquanto.

**Custo:** A é, em esforço, quase só o commit/push. B praticamente **dobra** a Fase de git, mais a
ferramenta de cifrar e a UI de divergência/bloqueio.

**DECIDIDO (2026-11-01): GitOps completo — opção B desde o início.** A opção A fica registrada
acima só como histórico da conversa. O "carregar árvore → validar → aplicar" é uma função própria,
compartilhada pela migração (R4), pela restauração e pelo `pull`.

**Ferramenta para escrever segredo à mão: um CLI separado, `rustploy-cli`** (nome provisório),
pequeno e feito para crescer com outros subcomandos depois. Começa com:
- `rustploy-cli secret encrypt` — lê o valor (stdin ou prompt, **nunca** como argumento, para não
  ir ao histórico do shell) e imprime `ENC[age:…]` cifrado para a chave **pública** do servidor
  (e a de recuperação, se houver). Dá para editar o YAML numa máquina sem a chave privada.
- `rustploy-cli secret decrypt` — com a identidade, decifra um valor (uso de recuperação).
- `rustploy-cli keygen` — gera a identidade `age` e a pública.
- A chave pública pode vir de `--recipient`, do `config.toml` ou de um endpoint do daemon.
Em aberto: viver no workspace do `rustploy-daemon` ou em repo/submodule próprio; reaproveitar o
código de `age` sem puxar o daemon inteiro.

### Conflito (decidido)
Se o `pull`/`fetch` mostrar que o histórico divergiu (não é fast-forward), o daemon **para a
sincronização, grava o motivo e avisa na GUI/webui**. Não faz merge nem rebase sozinho, e o
estado local segue funcionando normalmente. Retomar é ação manual (resolver no git e clicar em
"retomar").

Repositório sempre privado (o YAML expõe topologia e domínios mesmo sem segredos).

## R7. Pontos em aberto desta revisão

- ~~Remoção no git (prune)~~ — **decidido (2026-11-01):** só marcar "removido no git"; a remoção real é botão manual na GUI. Sem flag de prune por ora (opt-in no futuro, se fizer falta).
- Se a troca `Plain`→`Secret` dispara redeploy à toa (R5).
- Impacto na GUI/webui: telas que mostram `status` precisam da nova fonte; Settings ganha o painel do git nos **dois** clientes.
- `git` do sistema ou `git2`/`gix` (sem `reqwest`).
- **Decididos nesta rodada:** `rustploy-cli` mora no **workspace do daemon**; gatilho do `fetch` =
  intervalo fixo + botão (R6); varreduras de `daemon_settings` e `ServiceSpec` feitas (R2.1).
- Fases (ordem provável): repositório + transação com testes de queda → "aplicar árvore" →
  migração SQL→YAML → separar `service_runtime` → segredos inline + mover env↔secret/escopos →
  git/GitOps completo → CLI → UI. Detalhar depois da decisão do prune.


---

# R8. Dependências entre serviços e religamento completo (RASCUNHO EM DISCUSSÃO)

**Estado:** em conversa; nada implementado. Escrito em 2026-11-01. Esta seção **retoma e
atualiza** `docs/plano-dependencias-e-autostart.md` (2026-07-11, nunca executado). Aquele plano
continua certo na ideia, mas os caminhos citados nele estão velhos (`db/`→`store/`,
`deploy/`→`features/deploy/`, `crates/client`/TUI removido, handlers em `api/handlers/…`).
Conferi em 2026-11-01 que **nada dele foi feito**: não existe `depends_on` nem `desired_state` no
código, e os containers seguem criados com `RestartPolicy::NO` (`engine/containers/lifecycle.rs`).
Vale o que está aqui; o plano antigo fica como referência histórica do raciocínio.

## R8.1 O problema, em linguagem simples

Hoje, se o servidor reinicia, **nenhum serviço volta sozinho**, e o daemon não sabe de ordem
nenhuma: um banco e a API que depende dele são tratados como estranhos. Queremos dois
resultados:

1. **Reboot normal:** o que estava rodando sobe de novo, **na ordem certa** (banco antes da API),
   e o que o usuário parou à mão **continua parado**.
2. **Servidor perdido:** num servidor novo, restaurado do git (R6/§7), a pessoa clica em
   **uma vez** e o sistema inteiro volta, com cada serviço esperando as dependências ficarem prontas.

Os dois usam o mesmo motor: um **orquestrador de subida** que recebe um conjunto de serviços,
ordena pelo grafo de dependências e sobe em ondas.

## R8.2 Como isso muda com o YAML (a ligação com o resto deste plano)

Três dados diferentes, e cada um mora num lugar:

| Dado | O que é | Onde mora |
|---|---|---|
| `depends_on` | "este serviço precisa daqueles antes" — declarado por um humano | **YAML** (infra) |
| `autostart` | "este serviço deve subir no boot / na restauração" (padrão `true`) — declarado | **YAML** (infra) |
| `desired_state` | "estava rodando quando o servidor caiu?" — muda a cada Parar/Iniciar | **SQL** (runtime, junto de `status`) |

Por que separar `autostart` de `desired_state`: se o "parado à mão" morasse no YAML, **cada
Parar/Iniciar geraria um commit**, e o histórico do git viraria ruído. E se só existisse o
`desired_state` no SQL, um servidor restaurado do git (SQL vazio) não saberia o que subir. Juntos:

> **Sobe se `autostart = true` E não houver `desired_state = Stopped` gravado.**
> Reboot: o SQL existe, vale a parada manual. Servidor novo: o SQL está vazio, vale só o `autostart`.

Referências: no YAML, `depends_on` usa **nomes** de serviços do mesmo projeto (legível por
humanos e estável no git); a camada de repositório resolve para IDs ao carregar. A validação
(serviço inexistente, ciclo) faz parte do passo "validar" da transação (R3) e também do `pull`
(R6): **um commit externo que crie um ciclo é recusado**, como qualquer YAML inválido.

## R8.3 O modelo de dependência

- **Dentro do projeto:** `depends_on: [nomes]`. Grafo dirigido, sem ciclos (ordenação
  topológica, o `topo_sort` do plano antigo).
- **Entre projetos, só pelo banco compartilhado.** Hoje já existe `shared_access`: um servidor de
  banco compartilhado é conectado às redes dos projetos autorizados. Isso **já é** uma dependência
  real (o projeto B não funciona sem o banco do projeto A). Proposta: essas arestas são
  **implícitas**, derivadas de `shared_access`, sem o usuário declarar nada. Fora disso, nada de
  dependência entre projetos (simples e suficiente).
- **Serviço Compose é um nó só.** O `depends_on` interno do `docker-compose.yml` é do Docker; para
  nós, a stack inteira é um serviço, "pronto" quando o deploy termina (e o healthcheck, se houver).
- **Pronto significa o quê?** (é a decisão mais importante do modelo.) Opções:
  1. *Container rodando.* Rápido, mas a API sobe e falha porque o banco ainda está abrindo.
  2. *Healthcheck passou* (o `Healthcheck` do `ServiceSpec`); **sem healthcheck, cai para "a
     porta responde por TCP"**; sem porta, "container rodando". **Recomendado.**
  Com timeout por serviço (padrão sensato, configurável) para um banco lento não travar tudo.
- **Falha propaga:** se uma dependência não fica pronta, os dependentes ficam **bloqueados** com
  erro que diz qual dependência falhou (já decidido no plano antigo). Os independentes seguem.

## R8.4 O orquestrador de subida

1. Recebe um conjunto de serviços (alvo) e monta o grafo (arestas explícitas + as de `shared_access`).
2. Calcula **ondas**: a onda 1 são os sem dependência; a onda 2, os que só dependem da onda 1; etc.
3. Dentro de uma onda, sobe **em paralelo** (serviços independentes não precisam esperar um ao
   outro; num servidor com dez serviços isso corta muito o tempo). Entre ondas, espera ficar pronto.
4. Para cada serviço decide **como** subir:
   - container existe e roda → nada a fazer, conta como pronto;
   - container existe e está parado → `start`, depois espera o "pronto";
   - container **não existe** (servidor novo, ou removido) → dispara um **deploy completo** e
     espera terminar. Serviço Compose cai sempre aqui.
5. Reaproveita a **fila de deploys** que já existe (`features/deploy/queue.rs`), em vez de chamar
   o executor direto como o plano antigo propunha (aquilo é anterior à fila). A verificar na
   implementação como "esperar o deploy terminar" se encaixa na fila.
6. Publica progresso pelo barramento de eventos que a GUI/webui já usam ("subindo onda 2 de 4").

### Quando roda
- **No boot do daemon**, de forma síncrona antes de religar os loops `watchdog`/`reconcile`,
  para eliminar a corrida que o plano antigo achou (o `reconcile` rebaixa para `Stopped` o que
  acabou de sumir e o `watchdog` não enxerga mais). Hoje essa ordem fica em `bootstrap/mod.rs`
  (depois de `deploy::recovery::recover`) e `bootstrap/workers.rs`.
- **Por botão**, na GUI e na webui: **"Subir projeto"** (e "Subir tudo") usam o mesmo orquestrador.
  É o botão do cenário "servidor novo".
- Mantém `RestartPolicy::NO`: o rustploy é a única autoridade de quando e em que ordem religar;
  deixar o Docker religar sozinho (`unless-stopped`) criaria um segundo religamento sem ordem.

### Parar
Parar um serviço do qual outros dependem **avisa** quem depende dele (não bloqueia: a pessoa pode
querer mesmo). "Parar projeto/tudo" desce na **ordem inversa** (dependentes primeiro).

## R8.5 O que a restauração de um servidor novo ainda não resolve

Mesmo com o orquestrador, subir num servidor limpo esbarra em coisas **fora** do estado
declarativo. O plano precisa dizer isso com todas as letras:

1. **Imagens.** Não há containers nem imagens; cada serviço tem de ser **baixado ou
   construído** (git → build, registry → pull). Pode levar tempo e depender de rede/credencial.
2. **Registry embutido.** Os blobs ficam no SQL de runtime e **não** vão ao git. Um serviço cuja
   imagem vem do registry embutido do próprio servidor perdido **não consegue baixar**: precisa
   ser reconstruído a partir do código. Registrar como limitação.
3. **Volumes e dados dos bancos.** Fora do escopo (já dito no §10): o banco sobe **vazio**.
   O orquestrador liga o banco na ordem certa, mas não devolve os dados.
4. **Zips enviados** (`archive_id`) não existem mais (R2.1).
5. **Segredos.** Dependem da chave de criptografia/recuperação (R5); sem ela, o serviço falha
   com "secret ausente" (erro de deploy, já decidido).
6. **Portas externas** podem precisar de novo sorteio (R2.1).

### Decisões desta rodada (2026-11-01)
- **Valores concretos** (timeout de "pronto", tentativas, espera entre elas): **o assistente propõe defaults ao detalhar a Fase 3; o usuário ajusta.** Aceito.
- **"Pronto"** = healthcheck passou; sem healthcheck, a porta responde por TCP; sem porta, container
  rodando. Timeout por serviço, com padrão sensato e configurável. **Decidido.**
- **Paralelo dentro da onda**, sequencial entre ondas. **Decidido.**
- **Nova tentativa automática. Decidido:** um serviço bloqueado por dependência que falhou
  **tenta de novo sozinho** quando essa dependência ficar pronta. Detalhe de desenho:
  - É por **evento**, não por relógio: quando um serviço passa a "pronto", o orquestrador reavalia
    os dependentes bloqueados por ele e os sobe. Funciona também quando a dependência foi
    consertada à mão ou subiu por um deploy manual.
  - A própria dependência que falhou também é retentada com **espera crescente** (ex.: 5 s, 15 s,
    60 s, depois a cada alguns minutos), com teto de tentativas por rodada, para não martelar um
    banco quebrado nem encher o log. Depois do teto, vira erro visível e só o botão "Subir projeto"
    (ou um deploy manual) recomeça.
  - Serviço **parado à mão** nunca é religado por esse mecanismo (vale o `desired_state`).
  - Convive com o `watchdog` existente (que religa o que caiu em execução): o `watchdog` cuida de
    serviço que *estava no ar e caiu*; o orquestrador cuida de *subida em ordem*. A fronteira
    exata entre os dois é para detalhar na implementação.

## R8.6 Pontos em aberto (para a próxima conversa)

- ~~Arestas por `shared_access`~~ — **decidido: implícitas** (o orquestrador deriva; a tela do projeto mostra as derivadas, para não parecer mágica).
- ~~`autostart` + `desired_state`~~ — **decidido: separados** (R8.2).
- Timeout padrão de "pronto", número de tentativas e a espera entre elas (valores concretos).
- Onde ficam na UI: campo de dependências no formulário Advanced do serviço (GUI e webui),
  badge "Parado (manual)" × "Parado (aguardando boot)", botões "Subir projeto/tudo", e a ordem
  em ondas visível na tela do projeto.


---

# R9. Fases (RASCUNHO EM DISCUSSÃO)

**Estado:** proposta de 2026-11-01, em validação com o usuário. Cada fase é **entregável sozinha**
(compila, testa, pode virar release) e deixa o sistema funcionando. Ordem pensada para que o
risco maior (trocar a fonte da verdade) venha depois de o resto estar firme.

**Regra de ouro herdada do §8:** a feature só está pronta quando um **servidor limpo é
restaurado do git e volta inteiro** (Fase 8). Antes disso é promessa não testada.

| Fase | Entrega | Depende de | Risco |
|---|---|---|---|
| **0** | Higiene: `master.key` em 0600; **secret inexistente vira erro de deploy** (hoje `unwrap_or_default()`); lista de comandos proibidos no MCP (`rpc`) pronta, ainda vazia; **backup de env a cada 60 s desligado** (R9.0 item 4) | — | baixo |
| **1** | **Camada de repositório** (`InfraState` em memória, YAML determinístico, lock, staging + `COMMIT` + roll-forward, validação). Não ligada a nada ainda | — | médio |
| **2** | **Separar o runtime do serviço no SQL:** tabela `service_runtime` (`status`, `live_container_id`, `desired_state`). A fonte da verdade continua SQL | — | médio-alto |
| **3** | **Dependências + autostart:** `depends_on`/`autostart`, validação de grafo, arestas de `shared_access`, orquestrador em ondas, retry por evento, "Subir projeto/tudo", UI | 2 | médio |
| **4** | **Migração SQL → YAML** e troca da fonte da verdade (infra passa a vir do repositório) | 1, 2 | **alto** |
| **5** | **Criptografia e secrets:** identidade `age` no `config.toml`, `ENC[age:…]` inline, escopo de serviço, credenciais de banco como secret de serviço, ações de mover, ver valor (+ auditoria, fora do MCP), `rustploy-cli` | 4 | alto |
| **6** | **Git:** modo `commit` com auditoria; `push`; liga/desliga | 4, 5 | médio |
| **7** | **GitOps completo:** `fetch` por intervalo + "sincronizar agora", aplicar árvore, recusas, "removido no git", bloqueio por conflito | 6 | alto |
| **8** | **Restauração de ponta a ponta** em servidor limpo + "Subir tudo" | 3, 7 | médio |
| **9** | Docs (`rustploy-docs`, pt/en), empacotamento (`git` no `.deb`, permissões de `/etc/rustploy`), checklist de **Publish** | todas | baixo |

**Decidido (2026-11-01):** a Fase 3 (dependências + autostart) vem **antes** da migração para YAML (Fase 4).

**Decidido (2026-11-01):** a migração SQL→YAML da Fase 4 é **tudo de uma vez** (um release só, com backup do SQLite, ida e volta e teste sobre cópia do banco de produção). O daemon fala com o git pelo **binário `git` do sistema** (dependência de runtime do `.deb`/instalador; conferir se o `install.sh` já garante o `git`).

## R9.0 Decisões de desenho tomadas em 2026-11-01

1. **Autenticação no repo do estado: reaproveitar um `git_provider` já cadastrado.** A config
   `[state_git]` aponta para um `provider_id` + URL do repo. O push/fetch usa o token do provider
   (o OAuth do GitHub já pede `scope=repo`, que cobre escrita em repo privado). Cuidados: passar a
   credencial ao `git` por ambiente (`GIT_ASKPASS` ou `http.extraHeader`), **nunca** na URL do
   remote nem em argumento de processo (vazaria em `ps` e no `.git/config`); repositório sempre
   privado. **Decidido (Pergunta 9):** os tokens obtidos pelo login (`access_token_enc`, `refresh_token_enc`) **ficam só no SQL de runtime e nunca vão ao git**. O YAML leva a config do provider (tipo, `base_url`, `client_id`, `client_secret` cifrado). Na restauração o usuário **reautentica o provider** (clique OAuth ou cola um token) antes do primeiro clone — resolve o "ovo e a galinha" (o repo precisaria do token para ser clonado) e evita commit a cada renovação de token.
2. **Um repositório por servidor** (chiquitos e itemize têm repos distintos).
3. **Formato interno próprio** para o YAML de estado (com IDs, `ENC[...]`, `compose_project`); o
   manifesto `rustploy.yml` (IaC de hoje) fica como está, como formato portátil de "aplicar".
   Documentar os dois formatos e a diferença.
4. **Aposentar o backup de env a cada 60 s (`features/backup`) já na Fase 0**, antes de tudo.
   Consequência assumida: **entre a Fase 0 e a Fase 6 não há rede de segurança para "apaguei uma
   env sem querer"** (o git só protege depois). Cuidados: seguindo a regra "comentar, não remover",
   o worker é **desligado** e o código fica no lugar com `TODO`; a seção `[env_backup]` do
   `config.toml` precisa continuar sendo aceita no parse (que é tudo-ou-nada) para não quebrar quem a
   tem; o `ManifestExportAll` manual continua existindo. Vale um aviso nas notas de release.
5. **Rollback do daemon só dentro da janela do backup:** a atualização da Fase 4 guarda o SQLite
   anterior; voltar para o `.deb` antigo restaura esse backup e perde as edições feitas depois.
   Dizer isso com todas as letras nas notas de release da Fase 4.

## R9.1 O que cada fase precisa provar (testes de aceitação)

- **0:** rodar um deploy referenciando secret que não existe falha com mensagem que diz qual e onde.
- **1:** teste de **queda injetada**: matar o processo entre cada passo do fluxo de escrita (antes
  de gravar, depois de gravar, antes do `COMMIT`, depois do `COMMIT`, no meio do `rename`); em todos
  os casos o próximo boot termina em "estado antigo inteiro" ou "estado novo inteiro", nunca
  misturado. Dois exports seguidos geram bytes idênticos.
- **2:** nenhuma regressão nas telas que mostram status; reconciliação e watchdog leem a tabela nova.
- **3:** reboot simulado (`docker stop` + reiniciar o daemon): A sobe e fica pronto antes de B;
  serviço parado à mão não volta; dependência que falha bloqueia o dependente com o motivo e ele
  sobe sozinho quando a dependência é consertada; ciclo é recusado; aresta de `shared_access` conta.
- **4:** **ida e volta**: exporta o estado do SQL, relê o YAML e compara; aborta sem efeito se divergir.
  Roda sobre uma **cópia do banco de produção** (nunca só fixtures) antes de ir a release.
- **5:** teste de **vazamento**: varre o diretório inteiro procurando qualquer valor sensível
  conhecido em claro; chave errada falha sem escrever nada; mover env↔secret é atômico.
- **6:** ligar o git com um valor em claro é recusado e a UI lista quais; push que falha não derruba
  nem bloqueia o daemon.
- **7:** YAML inválido, ciclo, valor em claro e histórico divergente **não aplicam** e ficam
  visíveis; aplicar não gera commit de volta (sem laço); remoção só marca.
- **8:** servidor limpo + chave de recuperação + repo → tudo volta na ordem certa; o que não volta
  (dados, zips, blobs do registry) está listado na tela.

## R9.2 Observações de desenho

- **A Fase 3 não depende do YAML.** `depends_on` e `autostart` podem nascer no `spec` (JSON no
  SQL) e migrar com o resto na Fase 4. Isso entrega o religamento no reboot **cedo**, que é a dor
  de hoje, em vez de esperar a migração inteira.
- **A Fase 2 é pré-requisito silencioso.** Sem separar `service_runtime`, cada Parar/Iniciar
  mexeria numa linha que depois vai pro YAML.
- **A Fase 4 é a mais arriscada.** Mitigações: backup do SQLite antes, verificação de ida e volta,
  tabelas antigas mantidas como rollback por uma versão, e um release sozinho (sem misturar com
  segredos).
- **GUI e webui** acompanham cada fase que muda tela; "botão travado até a resposta" vale para
  toda ação nova.


---

# R10. Registro da implementação (noite de 2026-11-01 → 02) — LEIA AO ACORDAR

Implementação feita sem o usuário presente, sob estas regras: **nenhum commit, nenhum deploy, nenhum
publish**. Tudo está só na árvore de trabalho de `rustploy-daemon/` (e, quando indicado, de
`rustploy-shared/`). Cada item abaixo diz o que foi feito, como foi verificado e o que depende de uma pessoa.

## R10.1 Feito

### Fase 0 — higiene (concluída, testada)
- `master.key` nasce com 0600 e é corrigida para 0600 na subida (`platform/secrets.rs`; 2 testes).
- **Secret inexistente agora é erro** de deploy/job, com mensagem que diz qual secret, qual env e onde foi
  procurado (`features/deploy/env_resolve.rs`; `compose_flow.rs` deixou de engolir o erro; 2 testes).
  O `rollback` continua tolerante de propósito (está derrubando, não subindo).
- Gancho `proibido_ao_agente` no `rpc` do MCP (`api/agent/tools.rs`), **vazio por enquanto**; a Fase 5 coloca
  o `SecretReveal` nele.
- **Backup de env a cada 60 s desligado** (`bootstrap/workers.rs`, código mantido e comentado com `TODO`;
  a seção `[env_backup]` do `config.toml` segue sendo aceita).

### Fase 1 — camada de repositório (concluída, testada, **ainda não ligada a nada**)
- `store/infra/txn.rs`: `FileTree`/`Tx` com lock (`flock`), staging, `COMMIT`, roll-forward e descarte.
  Teste de **queda injetada em 8 pontos** (antes/depois de cada passo, no meio da aplicação) e de
  recuperação idempotente. `store/infra/yaml.rs`: YAML determinístico.
- Falta do que o plano da Fase 1 listava: o tipo `InfraState` (o estado tipado em memória) e a ligação
  com as entidades. Isso vem junto da Fase 4, porque depende de decidir o que cada entidade serializa.

### Fase 2 — runtime do serviço separado (concluída, testada)
- Tabela `service_runtime (service_id, status, live_container_id, desired_state, updated_at)` e migração
  idempotente com backfill (`store/mod.rs`). Leituras de `store/services.rs` fazem `LEFT JOIN`; a verdade
  de status passou para a tabela nova. **As colunas legadas de `service` seguem sendo escritas** (dupla
  escrita, `TODO` no código) para um downgrade do binário ler algo coerente.
- `desired_state` (`Running`/`Stopped`, padrão `Stopped`) com `set_desired_state`, `desired_state`,
  `get_desired_running`. Backfill: quem estava `Running/Degraded/Deploying/Queued/Stopping` vira `Running`.
- **Refinamento de desenho** (não estava no R8.2): `desired_state` tem **padrão `Stopped`**, e o **autostart
  do boot sobe só `desired_state = Running`**. Serviço criado e nunca iniciado não deve ser deployado
  sozinho num reboot. O caso "servidor novo restaurado do git" não usa o boot: usa o botão
  "Subir projeto/tudo", que sobe `autostart = true` ignorando o `desired_state`.

### Fase 3 — dependências + autostart (backend concluído e testado; **UI e protocolo pendentes**)
- **`rustploy-shared` (working tree, sem bump):** `ServiceSpec.depends_on: Vec<String>` (ids do mesmo projeto)
  e `ServiceSpec.autostart: bool` (padrão `true`), ambos com `serde(default)` — specs antigos no SQLite
  continuam legíveis (teste). **Isto exige publicar uma versão nova do shared** (ver R10.3).
- `features/deploy/deps.rs`: grafo em ondas, ciclo, transitivos, `validate`, dependentes diretos (12 testes).
- Validação ligada em `ServiceCreate`/`ServiceUpdate` (`features/services/{create,update}.rs`: dependência
  inexistente/repetida/de si mesmo/em ciclo é recusada) e **guarda de exclusão** em `delete.rs`
  (`DependencyInUse`). Caminhos que gravam spec sem passar por esses handlers (IaC `manifest_apply`) **não
  validam** ainda — o manifesto também ainda não tem `depends_on` (ver R10.3).
- `desired_state` gravado: `deploy::start` → `Running`; `deploy::stop` (quando deu certo) → `Stopped`.
- `features/deploy/autostart.rs`: orquestrador em ondas **com paralelismo dentro da onda**, bloqueio por
  dependência falha, **nova tentativa por rodadas com espera crescente** (5 s, 15 s, 60 s, 3 min, 5 min — valores
  *propostos*, em `Policy::default()`), dependências **implícitas** de `shared_access`, "pronto" = healthcheck
  (sem healthcheck: TCP na porta; sem porta: container rodando). 10 testes com um `Bringer` falso (ordem,
  paralelismo via `Barrier`, bloqueio, recuperação, esgotamento, ciclo, aresta implícita).
- Ligado no boot: `bootstrap/mod.rs` chama `deploy::autostart::spawn_boot` logo depois de `spawn_stateful`
  (precisa do worker da fila de deploys). Sobe `desired_state = Running` **e** `autostart = true`.
- O `Bringer` real (`DockerBringer`) **só foi compilado e revisado, não executado contra um Docker de
  verdade**: ligar containers parados e disparar deploy pela fila + esperar o desfecho. Precisa do teste
  manual descrito em R10.3.

## R10.2 Verificação
- Ao fim da Fase 3: `cargo test -p rustploy` com **346 testes verdes**; `cargo clippy --all-targets -D warnings`
  limpo; `cargo test` do `rustploy-shared` com 39 verdes. Tudo rodado **com o shared local**, via
  `--config 'patch.crates-io.rustploy-shared.path="../rustploy-shared"'` (a forma documentada no AGENTS.md; o
  `Cargo.toml` do daemon **não foi alterado** e o `Cargo.lock` foi restaurado).
- ⚠ **Sem esse `--config`, o daemon não compila neste momento**: o código já usa os campos novos do shared, que
  só existem no working tree do `rustploy-shared` até alguém publicar. Isso é esperado (R10.3, item 1).
- O crate `importer` do workspace do daemon fixa `rustploy-shared = 0.5.0` e **não foi tocado**.

## R10.3 O que precisa de você (em ordem de importância)

1. ~~Publicar o `rustploy-shared`~~ — **feito em 2026-11-02**: 0.11.0 no crates.io (commit e push no `main` do shared); o daemon já
   depende de `0.11.0` e compila sem `--config`. (Falta só commitar o daemon.)
2. **(ADIADO pelo usuário em 2026-11-02 — testar depois, antes de ir a release)** **Testar o `DockerBringer` de verdade** num Docker real (não rodei): criar projeto com `db` e `api`
   (`depends_on: [db]`), subir os dois, `docker stop` nos dois e `systemctl restart rustployd`; ver `db` ligar e ficar
   pronto antes da `api`. Também: parar a `api` pela UI e reiniciar o daemon (ela **não** pode voltar); e forçar o `db` a
   falhar (healthcheck impossível) e ver a `api` ficar `Error("dependência … não ficou pronta")`.
   Depois da atualização que cria `service_runtime`, o primeiro boot também religa tudo que estava `Running` (é o
   backfill); convém acompanhar esse primeiro boot.
3. ~~Revisar os valores de `Policy`~~ — **aprovados** pelo usuário em 2026-11-02 (5 s/15 s/60 s/3 min/5 min; pronto em 120 s; deploy em 30 min).
4a0. ~~Publicar shared 0.13.0 e o daemon~~ — **feito em 2026-11-02**: `rustploy-shared` 0.13.0 (commit `a1a08dc`) e `rustploy` (daemon) 0.1.21 (commit `5d2363c` no `main` do daemon) no crates.io. **Não feito:** tag/release do GitHub (os `.deb`), deploy das docs, `make index`, ponteiros dos submódulos no agregador. Os `.deb` só saem com a release; nenhum servidor foi atualizado.
4a. ~~Publicar o `rustploy-shared` 0.12.0~~ — **feito em 2026-11-02** (commit `6f6d85a` no `main` do shared, push e crates.io); o daemon já depende de 0.12.0 e compila sem `--config`. Texto original: **Publicar o `rustploy-shared` 0.12.0** (campos/variantes novos de secrets e `InfraAudit`; hoje o working tree do
   shared está com 0.11.0 + mudanças não commitadas, e o daemon só compila com `--config 'patch.crates-io.rustploy-shared.path="…"'`).
   Depois: subir a versão no `Cargo.toml` do daemon e `cargo update -p rustploy-shared`.
4. **Faltou na Fase 3 (precisa de decisão ou de teste visual seu):**
   - UI em **GUI e webui**: campo de dependências (checkboxes dos outros serviços do projeto) e `autostart` no form
     Advanced; badge "Parado (manual)" × "Parado (aguardando boot)" (exige expor `desired_state` no `Service` do shared);
     e as dependências **implícitas** de `shared_access` visíveis na tela do projeto.
   - Comandos novos no protocolo do shared: **"Subir projeto"** e **"Subir tudo"** (reusam `orchestrate`; ignoram o
     `desired_state` e sobem `autostart = true`). Hoje só existe o religamento do boot.
   - `depends_on` no manifesto IaC (`ServiceManifest`, por **nome**) e validação no `manifest_apply`.
   - Aviso ao parar um serviço do qual outros dependem (existe `deps::dependents_transitive`, sem uso ainda).
   - Rotas do ingress de um serviço religado com `docker start` só voltam no próximo ciclo do `reconcile` (30 s); se
     isso incomodar, chamar a restauração de rotas do serviço dentro do `DockerBringer`.
5. **Não foram feitas:** Fase 5 (criptografia/secrets/CLI — envolve UI e a decisão de
   onde a chave nasce na instalação), Fases 6 a 9.
6. ~~Notas de release~~ — rascunho em **R10.5**.
7. **Higiene do repo:** nada foi commitado. `git status` em `rustploy-daemon/` mostra os arquivos alterados;
   `rustploy-shared/` tem 3 arquivos alterados. O índice de código (`docs/indice/INDEX.md`) não foi regenerado
   (`make index` ao commitar). `docs/plano-dependencias-e-autostart.md` ganhou um aviso de "substituído por R8".

## R10.4 Fase 4 — migração SQL → YAML (2026-11-02, feita com o usuário presente)

**Dividida em duas metades, ambas feitas** (4a–4c: modelo e migração; 4d: trocar quem lê).

### 4a–4c: pronto (2026-11-02) (`store/infra/model.rs`, `store/infra/migrate.rs`)
- `InfraState` tipado: `settings`, `providers`, `projects` (cada projeto leva serviços, jobs, secrets, bancos
  gerenciados e acessos a banco compartilhado). Vai para disco como `MANIFEST.json` (schema 1), `settings.yml`,
  `providers.yml` e `projects/<id>/project.yml`. **Desvio do plano:** a pasta do projeto usa o **id**, não o nome
  (nome com espaço/acento, e renomear não pode virar "apagou + criou" no histórico do git).
- Fora do YAML, de propósito: status/container do serviço, `updated_at`, `last_run_at`/`next_run_at` dos jobs, tokens
  de login dos providers, chaves `*_last_run`/`warm`, `webhook_base_url`.
- `InfraState::validate()` (ids e nomes únicos, referências entre entidades), `write()` (só grava o que mudou, apaga
  projeto removido, tudo numa transação) e `read()` (recusa schema mais novo que o do daemon).
- `migrate::run(db, tree, db_path)`: cópia `VACUUM INTO` (`<db>.pre-yaml.bak`, nunca sobrescreve: `.1`, `.2`…),
  leitura do SQL, gravação, **ida e volta** (relê do disco e compara; divergiu → desfaz tudo e dá erro), e a marca
  `infra_yaml_migrated_at` em `daemon_settings`. Idempotente. (O plano dizia "linha em `migration`", mas essa tabela é
  o assistente de migração de banco, não controle de schema; por isso a marca ficou em `daemon_settings`.)
- Órfãos no SQL antigo (serviço sem projeto, job com gatilho inexistente, secret/banco/acesso apontando para o vazio)
  **não entram** no YAML e viram `warnings` do `Outcome` — nada é apagado do SQL.
- O diretório de infra passa a ser `0700`.
- **Verificação:** 9 testes novos (362 passam no total, mais 1 `#[ignore]` de dados reais; clippy limpo). Teste de dados reais (`#[ignore]`,
  `RUSTPLOY_INFRA_FIXTURE_DIR=<pasta> cargo test -p rustploy fixture_real -- --ignored --nocapture`) aplicando
  `rustploy-infra.zip` (8 projetos, 19 serviços → 11 arquivos, 80 KB) e `rustploy-infra2.zip` (1 projeto, 9 serviços →
  4 arquivos, 17 KB): ida e volta idêntica, sem avisos, e regravar não muda nenhum byte.
- **Limite honesto:** os zips são *manifestos* (IaC), não o SQLite. Eles provam o formato com serviços reais, mas
  **não** exercitam órfãos, secrets, jobs com histórico nem bancos gerenciados de produção. Isso só um `rustploy.db`
  real prova.

### 4d: feito (2026-11-02) — os módulos de `store/` passaram a ler e escrever no YAML
**Fundação.** `store::Db` deixou de ser um alias do pool: é uma struct com o `SqlitePool` e o `Repo`
(`store/infra/repo.rs`: `snapshot()` para ler, `update(|estado| …)` para escrever — copia, aplica,
valida tudo, grava a transação e só então troca o `Arc` em memória; erro em qualquer passo não deixa
rastro nem no disco nem na memória). `Db` faz `Deref` para o pool; onde o `sqlx` pede um executor
usa-se `db.pool()`. Os ~190 usos de `&Db` fora de `store/` não mudaram.

**Módulo a módulo** (cada um fechou com os testes verdes antes do próximo):
| Módulo | Onde mora agora | Observações |
|---|---|---|
| `daemon_settings` | 3 chaves de config → `settings.yml`; o resto (`*_last_run`, `warm`) segue no SQL | `sql_get` lê a tabela direto (usado pela migração) |
| `git_providers` | config → `providers.yml`; **tokens de login → nova tabela SQL `git_provider_token`** | tokens nunca vão ao git |
| `projects` | `projects/<id>/project.yml` | nome repetido é recusado pela validação; apagar o projeto leva o que é dele no YAML |
| `services` + `webhook_tokens` | spec, `compose_project` e webhook → YAML; status/container/`desired_state` → `service_runtime` (SQL) | `updated_at` agora vem do runtime (ou `created_at`); criar em projeto inexistente falha; o spec acompanha o projeto dono |
| `job` | definição → YAML; `last_run_at`/`next_run_at` → **nova tabela SQL `job_runtime`** | gatilho inexistente é recusado |
| `managed_database`, `shared_access` | dentro do projeto consumidor | `(servidor, nome)` único e servidor existente, validados |
| `secrets` (`SecretsManager`) | `ProjectInfra.secrets`, ainda cifrados com a passphrase | a Fase 5 troca para `ENC[age:…]` |

**Boot.** `store::connect` abre `<db_path>/infra` e roda `migrate::run` (uma vez). Se a marca
`infra_yaml_migrated_at` existe mas `infra/` está vazia/sumiu, o daemon **recusa subir** com mensagem
explicando (para não mostrar um servidor vazio que viraria o novo normal na próxima escrita). Instalação
nova (nada de infra no SQL) não gera `.pre-yaml.bak`. O seed de `service_runtime`/`job_runtime` a partir
das tabelas antigas só roda antes da marca.

**As tabelas antigas** (`project`, `service`, `job`, `secret`, `managed_database`, `shared_access`,
`webhook_token`, `git_provider`) **não são mais lidas nem escritas** e ficam congeladas como rollback.

**Verificação:** 373 testes passam (+2 `#[ignore]`), clippy `-D warnings` e `fmt` limpos. Dois testes
novos de ponta a ponta: banco antigo → `connect` → stores lendo do YAML (com tokens no SQL e runtime
semeado), e "`infra/` sumiu → não sobe". O teste de dados reais agora aplica o manifesto pelos stores,
**reescreve o resultado nas tabelas antigas** e migra: os dois zips (8 projetos/19 serviços e 1/9) voltam
idênticos e sem avisos.

**Pendências conhecidas da 4d (não resolvidas):**
1. ~~Snapshot de atualização incompleto~~ — **feito em 2026-11-02** (`features/update/apply.rs`): cada
   `pre-update-<versão>-<ts>.db` ganhou um par `pre-update-<versão>-<ts>.infra/` (0700) com o estado de infra
   gerado do estado **em memória** (sempre inteiro, nunca lido no meio de uma escrita). A poda trata o par como
   um snapshot (mantém 3). **Restaurar** (manual, como o `.db` já era; o rollback do helper só devolve binários):
   com o daemon parado, trocar `rustploy.db` pelo `.db` do snapshot e o conteúdo de `<db_path>/infra/` pelo do
   `.infra/` (apagar o `infra/` atual antes, inclusive `.lock`/`.staging`).
2. **Escritas bloqueiam a thread.** `Repo::update` usa `std::sync::Mutex` e `fsync` síncronos dentro de
   handlers async. São arquivos pequenos e escritas raras, mas é candidato a `spawn_blocking` se aparecer latência.
3. **`host_port` automático** (R2.1, item 1): a alocação ainda não roda *dentro* da mesma transação que grava o
   serviço. Fica para quando o GitOps (Fase 7) precisar.
4. **Senha do banco gerenciado em claro no YAML** até a Fase 5 (diretório 0700; sincronização com git não pode
   ser ligada antes).
5. ~~Testar contra um `rustploy.db` real~~ — **feito em 2026-11-02** com uma cópia do banco de `rustploy-ms.itemize.com.br`
   (24 MB), via `RUSTPLOY_PROD_DB=<arquivo> cargo test -p rustploy banco_de_producao -- --ignored --nocapture`:
   8 projetos, 19 serviços, 3 jobs, 2 bancos gerenciados, 2 acessos e 1 provider migraram com contagens idênticas;
   runtime preservado (17 Running → 17 com `desired_state = Running`); 11 arquivos / 86 KB; tokens de login do provider
   ausentes do YAML; reabrir o daemon não migra de novo e não muda o estado. **Achado:** 5 dos 18 tokens de webhook eram
   de serviços já apagados (o daemon antigo nunca limpava) — ficam fora do YAML e agora a migração **avisa** em vez de
   ignorar em silêncio. **Chiquitos (2026-11-02):** o mesmo teste com o banco do outro servidor (256 KB) — 1 projeto e 7 serviços, sem jobs, secrets,
   bancos, acessos, providers nem webhooks — migrou com contagens idênticas e sem avisos (4 arquivos, 7 KB); nenhum
   serviço estava `Running` nesse banco, então o religamento do boot não teria o que religar ali.
   **Dev (`rustploy-dev.itemize.com.br`, 2026-11-02):** banco de 7,9 MB — 1 projeto, 9 serviços, 2 providers e 5 webhooks
   (mais 2 tokens órfãos, avisados) migraram com contagens idênticas; 9 `Running` → 9 com `desired_state = Running`;
   4 arquivos, 19 KB; tokens de login ausentes do YAML; reabrir não muda o estado.

## R10.6 Fase 5 — criptografia, secrets com escopo e CLI (2026-11-02, backend)

**Pronto e testado** (412 testes passam, clippy `-D warnings` e fmt limpos; o daemon só compila com o `--config`
do `patch` até o shared ser publicado, ver "O que precisa de você" abaixo):

| Peça | O que é |
|---|---|
| **Identidade age** (`platform/secrets.rs`, `platform/config.rs`) | `[secrets] age_identity` (X25519) e `recovery_recipient` (pública, opcional) no `config.toml`. Valores gravados como `ENC[age:<base64>]`, cifrados para a chave do servidor **e** a de recuperação. O formato antigo (hex + passphrase da `master.key`) continua sendo lido. |
| **Primeira subida** | Sem identidade configurada, o daemon **gera uma e a grava no `config.toml`** (seção `[secrets]`, arquivo 0600, validado antes de trocar). Se o config não for gravável, grava em `age.key` ao lado da `master.key`. **Recusa subir** se precisaria gerar identidade nova mas já existem `ENC[age:…]` gravados (seria órfã-los). Avisa no log se `recovery_recipient` está vazia. |
| **Recifragem no boot** (`reencrypt_legacy`) | Secrets de projeto, `client_secret` e tokens de login dos providers, e webhooks (que a migração deixou em claro) passam para `ENC[age:…]`. Valor ilegível é mantido e avisado. Idempotente. |
| **Webhook cifrado** | `ServiceInfra.webhook_token` guarda `ENC[age:…]`; `webhook_tokens::get/upsert` agora recebem o `SecretsManager`. |
| **Secrets com escopo** | `ProjectInfra.secrets` (projeto) e `ServiceInfra.secrets` (serviço). Resolução de `Secret(nome)` numa env de serviço: **serviço, depois projeto**; env de projeto e job autônomo: só projeto; job com gatilho: serviço gatilho + projeto. Secret ausente = erro de deploy dizendo onde foi procurado. |
| **Mover** (`SecretsManager::move_secret`) | As 4 operações da R5, cada uma **uma transação do repositório**: env→secret, secret→env, secret de serviço→projeto, secret de projeto→serviço. Conflito de nome no destino falha sem mudar nada. |
| **Ver valor** | `reveal` grava `secret_audit` (quando/qual; nunca o valor) **antes** de devolver; se a auditoria falhar, o valor não sai. |
| **Fora do MCP** | `rpc` recusa `SecretReveal` (`proibido_ao_agente`). Limite honesto na R5: protege contra leitura acidental, não contra quem tem o token e fala HTTP direto. |
| **Protocolo (shared)** | `SecretSet/Delete/List` ganharam `service_id` opcional (`serde(default)`: clientes antigos seguem valendo); novos `SecretReveal`, `SecretMove { op: SecretMoveOp }`, `InfraAudit`; respostas `SecretValue`, `InfraAudit(Vec<PlaintextFinding>)`. |
| **Auditoria de valores em claro** (`store/infra/audit.rs`) | `plaintext_findings`: **certezas** (senha de banco gerenciado, webhook/secret/`client_secret` sem cifra) e **suspeitas** (envs com nome de segredo, URL com `user:senha@`, linhas de compose). Nunca devolve valores. Exposta como `InfraAudit`. É a trava da Fase 6. Rodada nos bancos reais: ~33 achados no dev, todos segredos de verdade (senhas MySQL/Mongo, chaves de API, tokens). **Heurística**: tem falso positivo e falso negativo (segredo com nome banal, ou dentro de Dockerfile/script, não é visto). |
| **Banco novo nasce com secret** | O `WizardCreate` de banco/broker move, depois de criar, cada env de credencial para um secret **do serviço** (`EnvToSecret`). Serviços existentes não são tocados. |
| **`rustploy-cli`** | Binário do pacote do daemon (`/usr/bin/rustploy-cli` no `.deb`): `keygen`, `secret encrypt` (valor só pelo stdin/prompt, **nunca** argumento; destinatários por `--recipient` ou do `config.toml`) e `secret decrypt`. Compartilha com o daemon o mesmo `platform/age_format.rs` (via `#[path]`), então o que o CLI cifra o daemon decifra. Testado de ponta a ponta com o binário real. |

**Consequências a saber (entram nas notas de release):**
- **O `config.toml` passa a ser alterado pelo daemon** (grava `age_identity`). Para o dpkg ele é um conffile
  modificado: em atualizações futuras do pacote, **se o `config.toml` empacotado mudar, o `apt` vai perguntar**
  "manter ou substituir". Por isso **não editei `packaging/config.toml`** (nem para comentar as chaves novas).
  Responder "substituir" apagaria a `age_identity`; o daemon então **recusa subir** em vez de gerar outra (caso haja
  segredos `ENC[age:…]`), e a identidade só se recupera de um backup do config ou da `recovery_recipient`.
- **Downgrade do binário depois da recifragem:** o binário antigo não lê `ENC[age:…]`. Ele volta a ler as tabelas
  antigas **congeladas** (estado do momento da migração), que ainda têm os valores no formato antigo.
- **Perder a `age_identity` sem `recovery_recipient` = perder todos os segredos.** Recomendar configurar a recuperação
  (`rustploy-cli keygen`, guardar a privada fora do servidor, pôr a pública em `recovery_recipient`).

**Não feito na Fase 5 (e por quê):**
1. **UI nos dois clientes** (GUI iced + webui): abas de secrets de serviço e de projeto, os botões de mover, "ver valor"
   mascarado com auto-ocultar (~15 s) e a lista do `InfraAudit`. Pede o shared publicado e teste visual seu.
2. **Senha de banco gerenciado** (`managed_database.password`, servidores compartilhados) segue em claro até o usuário
   mover; o fluxo de criação em `features/databases` ainda não a grava como secret. A auditoria a lista como certeza.
3. **`git.rs::resolve_clone_credentials`** (credencial de clone por nome de secret) só olha o escopo de projeto.
4. O snapshot do dashboard (`SecretList` dentro do `Snapshot`) lista só os secrets de projeto.
5. **Gate "recovery_recipient vazia" ao ligar o git** (a UI deve avisar): é da Fase 6.

## R10.8 Fase 6 — git: commit, push e a trava (2026-11-02, backend)

**Pronto e testado com o `git` de verdade e um repositório remoto local** (430 testes, clippy e fmt limpos):

| Peça | O que é |
|---|---|
| **Onde** | `features/state_git/` (`git.rs` = invólucro do binário `git`; `mod.rs` = `StateGit`). O repositório é o próprio `<db_path>/infra/` (`.git` dentro, `.gitignore` com `.lock` e `.staging/`). |
| **Modos** | `off` (padrão) · `commit` (local, sem rede) · `sync` (commit + `push`; o `fetch`/`pull` é a Fase 7). Guardados no **SQL** (`daemon_settings`: `state_git_mode/remote/branch/provider_id/audit_allow`), nunca no YAML que vai ao git. |
| **A trava** | Ligar (`off`→`commit`/`sync`) é **recusado** (`PlaintextFound`) se a auditoria achar valor em claro. E a auditoria roda **de novo a cada commit, dentro do lock de escrita** do repositório: se aparecer algo (ex.: "mover para env"), o commit é pulado, o status mostra `blocked_by_plaintext` e destrava sozinho quando limpar. Teste de que `git log -p --all` nunca contém o segredo, nem o que foi tentado e corrigido. |
| **Escape para falso positivo** | `InfraAuditAllow`: o usuário marca uma **suspeita** (ex.: `PASSWORD_MIN_LENGTH`) como "não é segredo". **Certezas** (senha de banco, webhook/secret sem cifra) **não** podem ser liberadas. O `InfraAudit` mostra `allowed` em cada item. |
| **Quando commita** | Worker no boot: a cada mudança do estado (`Repo::subscribe`), com **debounce de 10 s** (várias edições = 1 commit), a cada "sincronizar agora" e a cada 5 min para repetir o que falhou. **Sem diff, sem commit.** Assunto do commit com nomes legíveis ("Atualiza projeto Flow, configurações"). |
| **Lock** | `git` só lê o diretório com o lock de escrita tomado (`Repo::exclusive`): nunca vê uma transação pela metade. |
| **Credencial do push** | Token do `git_provider` (renovado uma vez se der 401/403), passado **só por variável de ambiente** (`http.extraHeader` via `GIT_CONFIG_*`); nunca na URL, em argumento (`ps`) nem em `.git/config`. O `git` ignora a config do usuário/sistema, não abre prompt e não usa credential helper. Qualquer texto que volta do `git` passa por `redact` (token e cabeçalho viram `***`). |
| **Remoto aceito** | `https://` sem usuário/senha embutidos (exige provider cadastrado), `file://` ou caminho absoluto (cópia local/teste). **`ssh://` e `git@host:` são recusados** — a credencial é o token do provider. |
| **Falha de push** | Não derruba nem bloqueia nada: o erro vai para `last_error`, o commit local fica, `needs_push` continua e tenta de novo. |
| **Protocolo (shared, não publicado)** | `StateGitStatus`, `StateGitConfigure { mode, remote?, branch?, provider_id? }`, `StateGitSyncNow`, `InfraAuditAllow`; `Response::StateGit(StateGitStatus)`; `PlaintextFinding.allowed`. |

**Limites a saber:**
- **Heurística da auditoria.** Quem tem muitas envs com nome de segredo em claro (o banco de dev tem ~33) **não consegue ligar** até movê-las para secret ou liberar as falsas. É por design (histórico limpo), mas é trabalho manual — a lista vem do `InfraAudit`.
- **Só HTTPS + token.** Deploy key SSH não é suportada.
- **`sync` ainda não faz `pull`/`fetch`** (Fase 7): se o remoto andou por fora, o `push` falha como não-fast-forward e o status mostra o erro; resolver é manual até a Fase 7.
- O ciclo roda dentro do lock de escrita só para `add`/`commit` (rápidos); o `push` também segura o lock enquanto envia (para não mudar a árvore no meio) — um remoto lento atrasa escritas de infra. Se incomodar, trocar por um snapshot do diretório.
- **Empacotamento (Fase 9):** o `git` precisa estar no servidor (`Depends`/`Recommends` no `.deb`, e conferir o `install.sh`). Sem `git`, o `StateGitStatus` mostra `git_available: false` e ligar é recusado.
- Pendência de **UI** (Settings → Sincronização com git: modo, repositório, provider, status, lista do `InfraAudit` com "liberar", aviso de `recovery_recipient` vazia) está em R10.7.

## R10.9 Fase 7 — GitOps: trazer de volta o que mudou por fora (2026-11-02, backend)

Em modo `sync`, cada ciclo agora faz: **commit local → `fetch` → comparar históricos → (enviar | aplicar | pausar)**.
27 testes de `state_git` com `git` de verdade, um repositório remoto local e um "colega" que clona, edita à mão e dá push.

| Situação | O que o daemon faz |
|---|---|
| Local e remoto iguais | nada |
| Só o local andou | `push` (como na Fase 6) |
| Só o remoto andou (e o local está limpo) | **lê a árvore do commit sem checkout**, valida tudo e aplica como **uma transação**; só depois alinha o git (`reset --hard`) |
| Os dois andaram, ou o repositório remoto tem história própria | **pausa a rede** com o motivo (`paused_reason`); o estado local segue funcionando e commitando. Retomar = `StateGitResume`, depois de resolver no repositório |
| Remoto ainda sem a branch | o primeiro `push` a cria |
| `fetch` falha (rede, credencial) | vai para `last_error` ("fetch falhou…"); nada quebra; tenta de novo no próximo ciclo |

**Validações antes de aplicar (qualquer falha = nada muda, commit recusado, `incoming_rejected` com sha e motivo):**
YAML/esquema inválido (inclui formato mais novo que o daemon) · referências quebradas (`InfraState::validate`) · **valor em
claro** (mesma auditoria da trava; a mensagem lista *onde*, nunca o valor) · **dependências** (inexistente/ciclo) · **todo
`ENC[age:…]` precisa decifrar com a identidade deste servidor** (senão o deploy falharia depois, longe de quem causou). Se
a árvore local tem mudança ainda não commitada, o pull **adia** (não é erro). Se o *topo* do remoto é válido, é ele que se
aplica — um commit ruim intermediário que já foi corrigido não trava nada.

**Sem laço (R6 item 3):** a árvore recebida é gravada **byte a byte** (`Repo::apply_files`), não reserializada. Teste: um
commit externo com comentário e formatação própria é aplicado, o arquivo em disco começa pelo comentário do colega, e
`cycle` repetido não cria commit nem push. Como o disco fica igual ao commit, o diff real é vazio.

**Remoção só marca (R6 item 5):** o que sumiu por commit externo (serviço ou projeto inteiro) sai do estado declarado, mas
o daemon **não toca em container, volume, `service_runtime` nem dados**. Fica uma linha em `state_git_removed` e em
`StateGitStatus.removed_in_git`; `StateGitDismissRemoval` esquece a marca. **Consequência:** sem o serviço no estado, a
rota do ingress some no próximo `reconcile` (o app sai do ar), embora o container continue vivo. Teste cobre serviço e
projeto inteiro.

**Declarado × rodando (R6 item 4):** serviço cujo `spec`/secrets mudou (ou cujo projeto mudou env/secrets) por commit externo
entra em `pending_deploy`; `deploy::start` limpa a marca. O daemon não redeploya sozinho.

**Gatilho (R6 item 7):** `fetch` periódico com `[state_git] fetch_interval_secs` no `config.toml` (padrão 60 s, mínimo
efetivo 10 s) **mais** "sincronizar agora" (`StateGitSyncNow`, que agora também faz fetch e aplicação). Não mexi no
`packaging/config.toml`: a chave é opcional.

**Protocolo (shared, não publicado → 0.14.0):** `StateGitResume`, `StateGitDismissRemoval { id }`; `StateGitStatus` ganhou
`last_fetch_at`, `ahead`, `behind`, `paused_reason`, `incoming_rejected`, `removed_in_git`, `pending_deploy` (todos com
`serde(default)`; status antigo continua legível).

**Acrescentado depois (2026-11-02, a pedido do usuário):**
- **Órfão visível na seção Docker/Podman.** `DockerContainerInfo.removed_in_git` marca o container cujo serviço sumiu por um commit
  externo: serviço normal pelo label `rustploy.service_id`, **Compose pelo nome da stack** (`com.docker.compose.project`, o mesmo nos dois
  engines porque o Rustploy usa o `docker-compose` também no Podman) — por isso a marca em `state_git_removed` passou a guardar o `compose_project`. Novo comando
  `StopContainer`, que **recusa** container de serviço vivo (senão o watchdog o religa). Remover continua sendo `RemoveContainer`.
- **Rollback devolve o serviço:** a marca "removido no git" some quando o serviço volta ao estado (antes ficava presa).
  O runtime nunca foi tocado, então o serviço volta já apontando para o mesmo container.
- **Divergência resolvida sem terminal:** `StateGitResolve { UseRemote | UseLocal }`, sempre com cópia de segurança do lado
  descartado (`rustploy-backup/local-<data>` no servidor; `rustploy-backup/remote-<data>` no remoto, com
  `--force-with-lease`). `UseRemote` passa o conteúdo do remoto pelas mesmas validações de um pull; um repositório que não é um
  estado do Rustploy **não** é adotado. Negado ao agente MCP.
- **Pausado = o daemon não mexe no git** (nem commita), para um merge por SSH não ser capturado pelo `git add -A` do daemon.
  As edições pela interface seguem valendo e são commitadas ao retomar.
- **"Retomar" relê a pasta e valida** (`Repo::reload_checked`): sem isso, depois de um `reset`/`merge` à mão a próxima escrita
  desfaria o que a pessoa resolveu (o daemon guarda o estado em memória). Pasta inválida (marcador de conflito…) = erro claro e
  continua pausado.
- **Documentação:** `rustploy-docs/src/content/docs/guias/sincronizacao-git.mdx` (+ `en/`), com o **procedimento manual por SSH**
  (`sudo -u rustploy git -C /var/lib/rustploy/db/infra …`, merge/ours/reset, conferir `status`, "Retomar").

**Limites a saber:**
1. **Remover do YAML tira o app do ar sem parar o container** (acima). Para limpar: Docker/Podman → Containers → Parar → Remover (backend pronto; **as telas estão em construção**).
2. **Resolver divergência**: os dois botões existem no backend (`StateGitResolve`); a **tela** está em construção. O procedimento manual por SSH está documentado.
3. **`host_port: 0` vindo de fora não é alocado** (R2.1 item 1): o valor entra como está.
4. **Editar `settings.yml` por fora** muda a configuração gravada, mas ACME/registry só relêem no próximo boot.
5. O `fetch` roda **fora** do lock de escrita; a aplicação e o `reset --hard` rodam **dentro** (segura escritas por instantes).
6. **Segredo escrito à mão exige o `rustploy-cli`** (ou a chave pública do servidor): é o preço de recusar valor em claro.
7. Só HTTPS+token (ou `file://`): o mesmo da Fase 6.
8. A UI (status, pausa, recusa com sha, removidos, pendentes de deploy, "retomar") está em R10.7.

## R10.10 Telas da sincronização (Fases 6 e 7) + teste com Podman real (2026-11-02)

**Telas, nos dois clientes** (o status e a auditoria vêm no **snapshot**: `snapshot.state_git` e `snapshot.infra_audit`, sem polling nem RPC próprio):
- **Aviso no topo** (pausada · commit do repositório recusado · commits suspensos por valor em claro) com botão "Abrir";
- **Settings → Sincronização / Sync**: modo (off/commit/sync), repositório, conta Git, branch, "Salvar e aplicar", "Sincronizar agora",
  situação (último commit/envio/consulta, à frente/atrás, erro), painel da **divergência** (os dois botões "Usar o do repositório" /
  "Usar o do servidor" + "Retomar"), commit **recusado** (sha e motivo), **removidos no git** (com "Esquecer") e a **auditoria**
  (com "Não é segredo" só nas suspeitas);
- **Runtime → Containers**: o órfão sai como "removido no git (serviço)" e, se roda, oferece **Parar** (depois, Remover);
- **Selo "mudança pendente de deploy"** no cartão do serviço e no detalhe.
Webui: `webui/app.js` (`stateGit*`), `index.html`, `format.js`, `locales/en.js`. GUI: `format/state_git.luau`, `handlers/state_git.luau`,
`home/settings/gitsync.gv`, `shell.gv` (aviso), `home/docker/containers.gv`, `components/service_card.gv`, `locales/*.json`.

**Testes das telas:** webui — 13 testes em Chrome headless (1 novo: `git_sync_screens_render_from_the_snapshot`, mede o que está
*visível*: 1 botão "Not a secret" e 1 "Stop"); GUI — `glacier test` 400+ verdes (8 novos do formatador, 1 novo de renderização da aba nos
vários estados, 1 ajustado) e `cargo test` (catálogos de idioma, manifesto das telas). **A GUI NÃO foi vista rodando** (sem
smoke visual/Xvfb): só avaliada pelo motor de testes. A webui **foi vista** no Chrome, contra o daemon real.

**Teste ponta a ponta com um engine REAL (Podman rootless) e o daemon de verdade** — o que antes só tinha testes com `git`:
- subir o daemon contra o Podman: migra o banco vazio, gera a identidade `age` no `config.toml` e abre a API (✔);
- assistente cria banco Postgres: **a senha nasceu como `!Secret` de serviço**, 0 ocorrências em claro nos arquivos, auditoria vazia (✔);
- `sync` com um repositório git local: commit, push, `fetch` (✔); commit externo com remoção de 2 serviços → aplicado, 2 marcas;
  mudança de porta por fora → `pending_deploy` (✔); **rollback com `git revert`** → serviços voltam, marcas somem, o `banco` reencontra o
  container (✔);
- inventário no Podman: órfão por `rustploy.service_id` **e** por `com.docker.compose.project`; container de serviço vivo **não**
  é marcado; `StopContainer` **recusa** o de serviço vivo e **para** o órfão; `RemoveContainer` o remove (✔);
- divergência: pausa; **"usar o do servidor"** sobrescreve o remoto e guarda o do colega em `rustploy-backup/remote-…` **no remoto**;
  **"usar o do repositório"** adota o remoto e guarda o local em `rustploy-backup/local-…` (✔); YAML quebrado e valor em claro vindos de
  fora são **recusados** com o sha e sem ecoar o valor (✔); o estado em YAML **sobrevive ao reinício** do daemon (✔).

**Defeitos achados por esses testes (todos corrigidos, com teste):**
1. a recusa de um commit **continuava visível** depois que o remoto voltava ao que o servidor já tinha;
2. o rótulo `io.podman.compose.project` que eu tinha acrescentado **não existe** neste projeto (o Rustploy usa o `docker-compose` nos
   dois engines): removido, e a documentação corrigida;
3. um `x-for` da webui sem encadeamento opcional **quebrava o console** quando o status ainda não tinha chegado (pegou: 12 testes headless);
4. `tools/busy_actions.py --check` procurava `*.gvb` e por isso **nunca checava nada**; passou a ler `*.gv` (e a lista ganhou as ações novas).

**Ainda NÃO testado** (segue em R10.7, item 3c): rollback com um **Compose de verdade** subido pelo daemon (usei containers rotulados à mão),
o `DockerBringer`, push/pull por **HTTPS com token** num GitHub/Gitea real, o procedimento por SSH da documentação num servidor real,
e a **GUI iced rodando**.

## R10.7 Pendências para fazer NO FIM (decisão do usuário, 2026-11-02)

Só depois de **todas** as fases estarem prontas. Não são bloqueio para continuar o desenvolvimento:

1. **Configurar `recovery_recipient` em produção** (usar `rustploy-cli keygen`, guardar a privada **fora do servidor**).
   Sem isso, perder o `config.toml` significa perder todos os segredos.
2. **O daemon passa a gravar o `config.toml`** (grava `age_identity`). Num `apt upgrade` futuro, se o pacote mudar esse
   arquivo, o apt pergunta "manter ou substituir": responder **"manter"**. Por isso o `packaging/config.toml` não foi tocado.
3. **Interface nos dois clientes (GUI iced e webui):** abas de secrets de serviço e de projeto, botões de mover, "ver
   valor" mascarado (auto-ocultar ~15 s) e a lista do `InfraAudit`; "botão travado até a resposta" (`busy_actions`) em
   toda ação nova. Precisa de teste visual do usuário.
3b. **Tela de Settings → Sincronização com git** (GUI e webui; inclui pausa/retomar, commit recusado com sha, removidos no git com "remover de verdade", pendentes de deploy), e `git` como dependência do `.deb`/`install.sh` (Fase 9).
3c. **TESTAR MANUALMENTE UM DIA, num engine real — Docker E Podman** (nada disto foi exercitado com um engine; os testes
   automáticos só usam o `git`):
   - o fluxo do container **órfão**: remover um serviço pelo Git → o container continua vivo → aparece em Docker/Podman →
     Containers com "removido no git" → Parar → Remover; conferir que o domínio sai do ar e que **Parar recusa** container
     de serviço vivo (inclusive Compose);
   - o **rollback**: `git revert` da remoção → o serviço volta com o mesmo id e **reata** o container (serviço normal por
     `rustploy.service_id`, Compose pelo `compose_project`, rótulo `com.docker.compose.project` nos dois engines); a rota volta em
     até 30 s e a marca some. E o contrário: recriar o serviço **à mão** (id novo) **não** reata o container antigo;
   - o `DockerBringer` do religamento no boot (item 2 de R10.3);
   - a divergência de ponta a ponta com um GitHub/Gitea de verdade (token do provider, `fetch`/`push` por HTTPS), os dois
     botões e o procedimento manual por SSH descrito em `rustploy-docs` (`guias/sincronizacao-git`).
4. Pendências já listadas antes e igualmente adiadas: UI da Fase 3 (dependências/autostart/badge/"Subir projeto"),
   teste do `DockerBringer` em Docker real, commit/`make index`/release.

## R10.5 Notas de release (rascunho — para a versão que juntar as Fases 0–3)

**Mudanças que o operador precisa saber**
- **Backup de env a cada 60 s foi desligado.** Entre esta versão e a sincronização com git (Fase 6) não há rede de
  segurança automática para "apaguei uma env sem querer"; o export manual (`ManifestExportAll`) segue valendo. A seção
  `[env_backup]` do `config.toml` continua aceita (e ignorada).
- **Secret inexistente agora falha o deploy** (e o job) com mensagem que diz qual secret, qual env e qual projeto. Antes
  virava string vazia em silêncio.
- **`master.key` passa a ter permissão 0600** (a existente é corrigida no primeiro boot).
- **Novo: dependências entre serviços e religamento no boot.** `depends_on` e `autostart` no `ServiceSpec`; o daemon
  religa os serviços que estavam ligados, em ordem de dependência e em paralelo dentro de cada nível, com retentativas.
  Serviço parado à mão não volta. Criar/editar recusa dependência inexistente ou em ciclo; apagar serviço de que outros
  dependem é bloqueado. *(Ainda sem campo na GUI/webui: só via API/protocolo.)*
- **Primeiro boot após atualizar:** tudo que estava `Running` é marcado como "deve estar ligado" (backfill de
  `desired_state`) e será religado se não estiver. Acompanhe esse boot.
- **Downgrade do binário:** as colunas antigas de `service` seguem sendo escritas, então o binário velho lê status
  coerente, mas **perde o `desired_state`** (e o religamento) até voltar a versão nova.
- **Requer `rustploy-shared` 0.11.0** (campos novos do `ServiceSpec`, com `serde(default)`: clientes antigos seguem funcionando).

**Segredos (Fase 5):** os segredos passam a ser cifrados com uma **identidade `age`** que o daemon gera na primeira
subida e grava em `[secrets] age_identity` no `config.toml` (0600); valores novos são `ENC[age:…]`. **Configure
`recovery_recipient`** e guarde a chave privada fora do servidor: sem ela, perder o `config.toml` = perder os segredos.
Secrets agora têm escopo de projeto **ou de serviço** (o de serviço vale primeiro), há botões de mover entre env/secret/escopo
e "ver valor" (com auditoria; fora do MCP), e bancos criados pelo assistente já nascem com a senha em secret. Novo binário
`rustploy-cli` (`keygen`, `secret encrypt|decrypt`). O daemon passa a **gravar** o `config.toml` (o `apt` pode perguntar
sobre o arquivo numa atualização futura do pacote: responda "manter"). Downgrade do binário não lê `ENC[age:…]`.

**Sincronização com git (Fase 6):** nova opção (desligada por padrão) que versiona `infra/` num repositório git e envia por HTTPS com o token de um provider cadastrado. **Só liga se não houver nenhum valor em claro** (veja `InfraAudit`); se aparecer um depois, os commits são suspensos até limpar. Requer o `git` instalado no servidor. `ssh://` não é suportado.

**GitOps (Fase 7):** em modo `sync` o daemon passa a fazer `fetch` (a cada `[state_git] fetch_interval_secs`, padrão 60 s) e **aplica** commits feitos por fora, depois de validar tudo (YAML, dependências, valor em claro, segredo que não decifra); o que falha é recusado e mostrado com o sha. Histórico divergido pausa a sincronização. **Remover um serviço no git o tira do estado e do ingress, mas não para o container.**

**Mudança grande, na Fase 4:** os dados de infra (projetos, serviços, jobs, secrets, bancos, providers, 3 configs)
passam a viver em `<db_path>/infra/*.yml` e o SQL guarda só runtime. A primeira subida migra sozinha e deixa
`rustploy.db.pre-yaml.bak`. **Rollback do binário só dentro da janela do backup** (restaurar o `.bak` perde as
edições feitas depois). Um `.deb` antigo continua lendo as tabelas antigas, que ficam **congeladas** no momento da migração.


### R10.11 Publicação e smoke visual da GUI (2026-11-02)

- Publicados no crates.io: `rustploy-shared` 0.14.0, `rustploy` (daemon) 0.1.22 e `rustploy-gui` 0.1.36.
  **Sem** release/tag do GitHub e **sem** deploy das docs (passos 8–10 do "Publish" ficam para depois).
- A GUI iced foi vista rodando (Xvfb, daemon de teste com remoto local, divergência provocada): aviso no topo,
  aba Sync, botão "Usar o do servidor" com confirmação, cópia `rustploy-backup/remote-…` criada no remoto.
- Defeito achado só ali: os painéis Situação/Divergência/Removidos **não renderizavam**, porque o `scrollable`
  aceita um filho só e eles eram irmãos do primeiro painel. Corrigido (coluna `form-stack`) e o teste de render
  agora confere os textos (`tela:texts()`), não só que renderiza.

### R10.12 Servidor chiquitos migrado (2026-11-02)

Primeiro servidor real migrado para o estado em YAML (daemon 0.1.20 → 0.1.25, via `make install-rustploy`).

- **Migração SQL → YAML:** `projects=1 services=7`, backup `rustploy.db.pre-yaml.bak`, serviços intactos.
- **Chaves:** identidade `age` ficou em `/etc/rustploy/age.key` (a gravação no `config.toml` falhou no 1º boot, motivo não achado;
  o log agora mostra a causa completa). `recovery_recipient` configurado; **teste da chave de recuperação: deu certo**
  (relatado pelo usuário: decifrou um `ENC[age:…]` com a chave privada via `rustploy-cli secret decrypt -i`).
  Backup do `age.key` fora do servidor: feito. `master.key`: apagado (daemon 0.1.24+ não o cria mais).
- **Secrets:** `InfraAudit` vazio. `cache` lê `REDIS_PASSWORD` de secret de **serviço**; `db` (recriado pelo usuário às 17:26,
  id novo) lê `POSTGRES_PASSWORD` de secret de **projeto**. Deploy de teste dos dois: `Live`, containers de pé.
- **Pendente:** ligar o sync (git instalado; falta conta/PAT e repositório); helper de firewall (`rustployd-fw`) falhou ao liberar a
  porta 20000 ("Connection reset by peer", bolinha vermelha no serviço) — hipótese: `ReadWritePaths=/etc/ufw /lib/ufw` sem `-`
  quebra o namespace do serviço quando o `ufw` não está instalado (226/NAMESPACE). Falta ver `systemctl status rustployd-fw.service`.
- **Achados do caminho:** `ubuntu-latest` virou 26.04 e quebrou o build do Windows (fixado em 24.04); `make install-rustploy` apaga
  os journals do sistema todo (`journalctl --vacuum`); a pergunta de motor do `install.sh` não espera resposta num shell remoto
  (nushell) — usar `ENGINE=podman YES=1`.
