# Copiar um serviço de um servidor para outro

> **Status (2026-10-09): PROJETO, nada implementado.** Este doc existe para você
> aprovar (ou corrigir) o desenho antes de qualquer código. As decisões já
> tomadas e o que ainda está em aberto estão na seção 8.

## 1. O que você quer fazer

1. Logar no servidor **A**, abrir o serviço **M**.
2. **Copiar** o M.
3. Desconectar do A, conectar no servidor **B**.
4. Criar um serviço **N** no B a partir da cópia do M.

Os dois servidores são daemons independentes, cada um com o seu banco. Eles não
se enxergam. A única coisa que atravessa de um para o outro é **o que o cliente
(GUI ou webui) carrega na mão** — por isso a pergunta "um YAML do serviço, ou um
arquivo?" é na verdade a pergunta central: **em que formato a cópia viaja?**

## 2. O que já existe (e por que não serve do jeito que está)

O rustploy já tem Infra-as-Code: `ManifestExportAll` gera um zip com
`rustploy.yml` + um TOML de variáveis, e `ManifestImport` aplica isso no destino.
Pontos bons: o formato de serviço (`ServiceManifest`) já existe, já redige env
vars para `${VAR}`, já referencia Git provider **por nome** (não por id interno).

Mas é feito para **o servidor inteiro, reconciliando**:

- exporta *todos* os projetos; não há "só o serviço M";
- o import **reconcilia projeto** (cria/atualiza/e, com `prune`, **apaga** o que
  não está no arquivo) — perigoso para "quero só mais um serviço";
- exige o projeto no próprio manifesto, e você quer escolher o projeto de
  destino na hora;
- o `ServiceManifest` **perde coisas**: `domains` (várias rotas de domínio),
  `env_comments`, pré-deploy e serviço de origem **zip** (vira registry vazio).
  Hoje isso é um TODO conhecido no `to_spec`.

Conclusão: reaproveitar o **formato de serviço** e as peças de redação/interpolação,
mas com um **par de comandos novo e de escopo um-serviço**, sem reconciliar nada.

## 3. A resposta curta: um arquivo, um serviço

O formato é **um arquivo YAML por serviço** ("pacote de serviço"). Por quê:

- **Funciona entre máquinas e entre clientes**: você exporta na GUI do notebook,
  importa na webui de outro lugar. Um estado "na memória do app" não sobrevive a
  isso.
- **Não vaza para a RAM da sessão**: hoje o logout apaga o `ctx` inteiro de
  propósito (nada do servidor anterior pode sobreviver). Uma "área de
  transferência interna" que atravessa o desconectar iria contra essa regra.
  Com arquivo, o dado está onde *você* decidiu guardá-lo.
- **Versionável e revisável**: dá para abrir, ler, editar (trocar domínio, imagem,
  porta) e commitar num repo.
- **Texto também**: o mesmo YAML pode ser copiado como texto para o clipboard do
  sistema / colado numa caixa de texto, para quem não quer passar por arquivo.

Esboço do arquivo:

```yaml
apiVersion: rustploy/v1
kind: Service
origin:                       # informativo; o import não depende disso
  server: https://rustploy.a.tech
  project: Flow
  exported_at: 2026-10-09T13:40:00Z
  daemon: 0.1.0
service:                      # = ServiceManifest, ampliado (seção 5)
  name: api
  source:
    git: { url: https://github.com/acme/flow.git, branch: development,
           dockerfile: dockerfiles/api.dockerfile, provider: github-acme }
  port: 8888
  domains:
    - { domain: api.a.tech, port: 8888, tls: true }
  env:
    DATABASE_URL: postgres://...      # valor literal, ou ${DATABASE_URL} se exportado sem valores
    API_KEY: secret:API_KEY           # referência a secret — nunca decifrada
  volumes: ["data:/var/lib/app"]
  healthcheck: { type: http, path: /health, status: 200 }
project_env:                   # variáveis do PROJETO que o usuário marcou (seção 4)
  REDIS_URL: redis://cache:6379
  SENTRY_DSN: secret:SENTRY_DSN       # referência — nunca decifrada
```

Um único arquivo, sem TOML ao lado (como o IaC faz): "com valores" = literais no
próprio `env`/`project_env`; "sem valores" = `${CHAVE}` no lugar (o import avisa
que faltam). `secret:NOME` é sempre referência.

## 4. O caminho do usuário

**Por que as variáveis do projeto entram na conversa.** Em runtime o container
recebe **todas** as variáveis do projeto **mais** as do serviço (a do serviço
vence em caso de chave repetida — ver `deploy/env_resolve.rs`). Ou seja: o app
pode depender de uma variável que só existe no projeto, e **o rustploy não tem
como saber quais** (o código do app é quem lê). Se o export levasse só as do
serviço, o N chegaria no B sem variáveis de que o M precisava e quebraria.
Levar *todas* as do projeto também é ruim: o projeto pode ter dezenas de
variáveis de outros serviços (e segredos que não têm nada a ver com o M).
Por isso a escolha fica com você, numa tela de checkboxes.

**No A (serviço M, dentro do projeto dele):** botão **"Exportar serviço…"** na tela do serviço (ao lado de
Deploy/Reload/Rebuild/Stop ou num menu). Abre uma tela com:

1. **Variáveis do serviço** — todas, já marcadas.
2. **Variáveis do projeto** — a lista completa, cada uma com checkbox, nome, e
   tipo (texto ou `secret:`). Vêm **pré-marcadas só as que provavelmente o M usa**:
   as citadas por nome como `${VAR}` no env do serviço ou no compose dele. As
   demais aparecem desmarcadas, com "marcar todas / desmarcar todas" e uma busca.
   (É uma sugestão, não uma garantia — por isso a decisão final é sua.)
3. Uma caixa **"Incluir os valores"** (padrão ligada) com o aviso de que o arquivo
   passa a conter segredos. Desligada, o arquivo leva só os nomes das variáveis.
4. Salvar `.yml` ou copiar como texto.

**No B:** abra o **projeto** onde quer o serviço e, em **"Novo serviço"**, use a
entrada nova **"Importar de arquivo/YAML"**.
Passos:

1. Escolhe o arquivo (ou cola o texto).
2. O **projeto de destino é o projeto em que você está** — o import é uma criação
   de serviço dentro dele, como qualquer outra. Não há escolha de projeto e,
   portanto, nenhuma chance de falhar por "projeto inexistente".
3. O cliente pede uma **pré-visualização** ao daemon (`dry_run`): o que vai ser
   criado e **o que não pôde ser trazido** (seção 6), em linguagem simples.
4. **Variáveis do projeto trazidas**, uma linha cada, com o estado no B:
   *nova* (será criada no projeto), *igual* (já existe com o mesmo valor, nada a
   fazer) ou *em conflito* (já existe com outro valor). Cada linha tem checkbox e,
   nos conflitos, três escolhas: **manter a do B** (padrão), **sobrescrever a do
   projeto do B** (mexe em quem já roda lá — pede confirmação), ou **só para este
   serviço** (grava a do arquivo no env do N, que vence a do projeto, sem tocar
   no projeto).
5. Escolhe o nome do N (pré-preenchido; se já existe no projeto, sugere
   `api-copia`).
6. Confirma. O serviço é **criado e NÃO é deployado** — você revisa e clica
   Deploy. (Há um checkbox "deployar em seguida", desligado por padrão.)

## 5. Peças a construir

**`rustploy-shared`**
- `ServiceBundle { apiVersion, kind, origin, service: ServiceManifest,
  project_env }` (`service_bundle.rs`). O `shared` não faz parse de YAML (não
  tem a dependência): só o formato, `validate()` e as conversões; quem lê/escreve
  YAML é o daemon.
- `suggested_project_vars(spec, chaves)`: a heurística de pré-marcação, pura e
  testada (aqui, para ser uma só para GUI e webui).
- Ampliar `ServiceManifest` (também melhora o IaC atual): `domains` (várias
  rotas), `env_comments`. Hoje o `from_spec/to_spec` os descarta.
- Novos tipos de protocolo (JSON, externally-tagged como os demais):
  - `Command::ServiceExportPlan { service_id }` →
    `Response::ServiceExportPlan { service_env, project_env }`: as variáveis do
    serviço e do projeto (nome, tipo plain/secret, e `suggested: bool` pela
    heurística `${VAR}` — ela mora no daemon para GUI e webui não duplicarem). Não
    devolve valores de `secret:`.
  - `Command::ServiceExport { service_id, include_values: bool,
    project_env_keys: Vec<String> }` → `Response::ServiceBundle { yaml }`
  - `Command::ServiceImport { yaml, project_id (obrigatório: o projeto aberto), name?,
    on_conflict, drop: {...}, secrets: {nome: valor},
    project_env: {CHAVE: manter | sobrescrever | so_servico | ignorar},
    deploy: bool, dry_run: bool }` → `Response::ServiceImportReport {
    service_id?, actions, warnings, missing_secrets, missing_vars,
    project_env: [{chave, estado: nova|igual|conflito}] }`

**`rustploy-daemon`**
- `service_export.rs`: monta o bundle a partir do `ServiceSpec` (reaproveita
  `from_spec` e `redact_env_map`).
- `service_import.rs`: parse → valida → resolve (projeto, provider por nome,
  secrets) → `dry_run` devolve só o relatório → senão cria via o mesmo caminho do
  `ServiceCreate` (porta externa **sempre** alocada de novo, ver seção 6).

**GUI e webui** (as duas — regra do projeto)
- Tela do serviço: "Exportar serviço…" (com a tela de checkboxes).
- "Novo serviço": "Importar…" com os 6 passos acima.
- Ambos usam o travamento de botão até a resposta
  (`docs`/AGENTS.md, "Botão que dispara requisição fica travado").

## 6. O que NÃO atravessa — e o que fazemos com cada coisa

Este é o ponto mais importante: copiar a **configuração** não copia o **mundo**
em volta dela. Cada item abaixo vira uma linha do relatório de pré-visualização,
nunca uma falha silenciosa.

| Item | Por que não vai junto | O que o import faz |
|---|---|---|
| **Dados** (volumes, conteúdo do banco) | só a configuração é copiada | traz a definição do volume vazio; avisa "dados não foram copiados" |
| **Secrets** (`secret:NOME`) | o daemon nunca decifra para o cliente | mantém a referência; lista **os que faltam no projeto do B** e deixa você informar o valor (`SecretSet`) ou seguir sem |
| **Git provider** | provider é conexão do A (OAuth/PAT) | referenciado por **nome**; se o B não tem, avisa e deixa escolher outro provider do B (ou sem) |
| **Variáveis do projeto** | o B tem o próprio projeto, com as próprias variáveis | só as marcadas viajam; no B cada uma é *nova*, *igual* ou *em conflito* e **nada é sobrescrito sem você escolher** (seção 4, passo 4) |
| **Porta externa** (`host_port`) | pode estar ocupada no B | nunca copia: o B aloca uma nova da faixa dele |
| **Domínios / TLS** | DNS aponta para o A | traz; avisa que o DNS precisa apontar para o B; opção "não trazer domínios" |
| **Pré-deploy checks** | ids de Jobs são do A | descarta, com aviso |
| **Servidor de banco compartilhado** (`shared`) e bancos gerenciados | dependem de projetos/usuários do A | descarta o flag, com aviso |
| **Origem por zip** (upload) | o zip está guardado no daemon A | **v1: recusa** com mensagem clara ("reenvie o zip no B"); v2 pode embutir o zip em base64 |
| **Serviço Compose de banco/broker** | o DNS interno é `rp_<nome>`; renomear quebra a URL interna | se o nome mudar, o import **avisa/impede** o rename (a conferir no código ao implementar) |

## 7. Segurança

- O arquivo **pode conter segredos** (valores de env vars, se você optar por
  incluí-los). Texto na tela e no export avisa isso, explícito.
- `secret:NOME` **nunca** é decifrado: só a referência viaja (tanto no serviço
  quanto no projeto).
- Variáveis do projeto **desmarcadas** nem entram no arquivo: o que você não
  marca não vaza.
- O import **nunca apaga nem sobrescreve** nada: se o nome já existe no projeto,
  é erro/renomear (padrão), não "atualizar". Atualizar um existente seria uma
  opção separada e explícita, fora da v1.
- Nada é deployado sem clique.
- O arquivo de import vem de fora: o parse é estrito (campo desconhecido = erro),
  com limite de tamanho, e o compose embutido passa pelas mesmas validações do
  `ServiceCreate`.

## 8. Decisões

**Tomadas (2026-10-09):**

1. **Valores das env vars:** caixa no export, **padrão ligada**, com aviso de que o
   arquivo contém segredos. Vale para as variáveis do serviço **e** do projeto.
2. **Variáveis do projeto:** também exportadas, escolhidas numa **tela de
   checkboxes** (porque o rustploy não sabe quais o app lê). Pré-marcadas pela
   heurística `${VAR}`; o resto é com você.
3. **Domínios:** trazer e avisar que o DNS precisa apontar para o B (com opção de
   não trazer).
4. **Escopo da v1:** só arquivo/texto. Sem atalho "copiar/colar" em memória.

**Ainda em aberto (tenho um padrão, diga se discorda):**

- **Heurística de pré-marcação.** Padrão: marcar a variável do projeto se o nome
  aparece como `${NOME}` / `$NOME` no env do serviço ou no compose. Não adivinha o
  que o código do app lê; é só ponto de partida.
- **Conflito de variável do projeto no import.** Padrão: **manter a do B**. As
  outras duas (sobrescrever; só para este serviço) ficam a um clique.

## 9. Ordem de trabalho (aprovado em 2026-10-09)

> **Passo 1 feito (2026-10-09)** — `ServiceBundle` e `ServiceManifest` ampliado.
> **Passo 2 feito (2026-10-09)** — `ServiceExportPlan`, `ServiceExport` e
> `ServiceImport` no daemon (`api/handlers/service_export.rs`, `service_import.rs`),
> com testes fim-a-fim A→B. **`rustploy-shared` 0.3.0 publicado em 2026-10-09** (daemon e GUI já o usam; antes disso só compilavam com o `patch` local): o daemon
> só compila com `--config 'patch.crates-io.rustploy-shared.path="../rustploy-shared"'`
> até a publicação (bump 0.3.0: `ServiceManifest` ganhou campos públicos).
>
> **Correção (2026-10-09): o import é sempre dentro de um projeto que já existe.**
> Saiu tudo que tratava de criar projeto no import (`new_project_name`,
> `project_created`, `ProjectExists`, desfazer projeto criado): `project_id` é
> obrigatório e é o projeto da tela aberta.
>
> **Passo 3 feito (2026-10-09) — webui:** `screens/service_bundle.js` (janela "Exportar
> serviço" com os checkboxes, e o passo "Importar" do wizard "Novo serviço"),
> validado no Chrome contra dois daemons de teste. **Passo 4 feito — GUI:**
> `handlers/bundle.luau` + `service_export_window.{gvb,luau}` (exportar) e
> `handlers/bundle_import.luau` + passo `import_form` em `new_service.gvb`
> (importar). Validado com testes de renderização e com os handlers Luau reais
> rodando contra um daemon de verdade; **o visual da GUI numa janela real não foi
> conferido** (o app instalado segura a trava de instância única).
>
> Diferenças em relação ao desenho acima: o `shared` não lê YAML (o daemon lê);
> `host_port` do pacote vira `0` ("alocar") em vez de sumir; `ServiceExportPlan`
> devolve também `blocked` (origem por zip); um export sem valores pede de volta
> TODO valor de env do serviço (os do projeto também), e o `${OUTRA}` escrito pelo
> usuário só é tratado como marcador quando está sob a própria chave.

1. `shared`: ampliar `ServiceManifest` (`domains`, `env_comments`) + `ServiceBundle`
   (com `project_env`) + testes de ida e volta (spec → bundle → spec igual).
2. `daemon`: `ServiceExportPlan`, `ServiceExport` e `ServiceImport` (com
   `dry_run`, política de conflito das variáveis do projeto) + testes.
3. webui: exportar e importar (mais fácil de iterar/testar no navegador).
4. GUI: o mesmo, com `save_file`/`open_file` do Luau.
5. Paridade (índice `comandos.md`), `make index`, doc de uso no AGENTS.md.

Cada passo é commitável e testável sozinho; o 1 já melhora o IaC atual.
