# Plano: índice de código para busca barata

> **Status:** fases 1 a 4 implementadas em 2026-09-29 (`tools/indexer`, saída
> em `docs/indice/`, `make index`); fases 5 e 6 (opcionais) pendentes.

## 1. O problema, em linguagem simples

Hoje, para descobrir "onde fica a lógica que faz X", um agente (Claude,
Gemini…) tem basicamente duas ferramentas: `grep` e ler arquivo. As duas
custam caro aqui:

- **Ler arquivo** é o mais caro. O `deploy/executor.rs` tem ~2.000 linhas,
  o que dá cerca de 25 mil tokens. Muitas vezes o agente lê um arquivo
  inteiro só para descobrir que a coisa não estava ali.
- **grep** é barato quando você já sabe o nome ("`fn recover`"), mas caro
  quando você só sabe a *responsabilidade* ("quem poda rota de ingress
  órfã?"). Aí o agente tenta palavras-chave, recebe dezenas de linhas
  soltas e vai lendo arquivo por arquivo.

O que falta é o equivalente ao **sumário de um livro**: um documento
pequeno que responde "em que capítulo está isso?" sem abrir o livro.

Números de hoje, para dar ideia do tamanho:

| O quê | Quantidade |
|---|---|
| Arquivos `.rs` | 157 (~34 mil linhas) |
| Funções/métodos Rust | ~1.170 |
| structs/enums/traits | ~190 |
| Funções Luau + JS | ~310 |
| Arquivos `.rs` com `//!` (descrição do módulo) | 55 de 157 |
| Variantes de `Command` no dispatch | ~170 |

## 2. A restrição: nada de número de linha

Você pediu, com razão, que o índice **não** dependa de número de linha.
Qualquer edição acima de uma função mudaria o número dela, e o índice
ficaria mentindo depois de quase todo commit.

A solução é endereçar cada coisa **pelo nome**, não pela posição:

```
crates/daemon/src/deploy/recovery.rs
  fn recover(db, docker, …) — reconcilia containers com o banco no boot
```

O nome de uma função muda muito menos do que a linha dela. E quando o
agente precisa da linha exata, ela custa quase nada:

```
grep -n "fn recover" crates/daemon/src/deploy/recovery.rs   → ~20 tokens
```

Depois ele lê só aquele trecho (`Read` com `offset`/`limit`), e não o
arquivo inteiro. O índice fica com o **"o quê" e o "onde (arquivo)"**, e o
grep resolve o **"em que linha"** na hora. Assim o índice só precisa ser
atualizado quando alguém cria, renomeia ou apaga um símbolo, e não quando
alguém edita o corpo de uma função.

## 3. Como a busca vai funcionar (o fluxo desejado)

Três degraus, cada um mais estreito que o anterior:

1. **Mapa geral** (`docs/indice/INDEX.md`, cerca de 2–3 mil tokens): a
   árvore de diretórios e arquivos, com **uma linha** dizendo a
   responsabilidade de cada um. Também diz qual arquivo do degrau 2 abrir.
2. **Índice do pedaço** (`docs/indice/<área>.md`, cerca de 2–5 mil tokens
   cada): para cada arquivo daquela área, os tipos, os métodos agrupados
   por `impl` e as funções livres, com a primeira linha da doc de cada um.
3. **Pulo cirúrgico**: `grep -n` pelo nome exato e depois `Read` de umas
   50–150 linhas.

Custo típico para achar uma responsabilidade: **~5–8 mil tokens**, contra
20–60 mil tokens hoje, quando se lê um ou dois arquivos grandes às cegas.
E quando o agente já sabe em que área está, ele pula o degrau 1.

### Por que dividir em vários arquivos e não num só?

Um índice único de tudo daria uns 25–30 mil tokens, o mesmo custo de ler
o `executor.rs`. Dividido por área, o agente paga só pela área que
interessa. As áreas propostas seguem as fronteiras que já existem no
código:

| Arquivo do índice | Cobre |
|---|---|
| `INDEX.md` | árvore inteira + 1 linha por arquivo/diretório |
| `shared.md` | `crates/shared` (modelos, protocolo, manifest) |
| `daemon-api.md` | `crates/daemon/src/api` (http_api, handlers) |
| `daemon-deploy.md` | `deploy/`, `jobs/`, `maintenance/` |
| `daemon-db.md` | `db/` |
| `daemon-docker.md` | `docker/`, `ingress/` |
| `daemon-registry.md` | `registry/`, `git_providers/` |
| `daemon-core.md` | resto do daemon (main, bins, event bus, secrets…) |
| `gui.md` | `crates/rustploy-gui/src` (Rust) |
| `gui-handlers.md` | `views/scripts/handlers/` (Luau) |
| `gui-scripts.md` | resto de `views/scripts/` (estado, rede, `fmt/`, janelas) |
| `gui-views.md` | `.gv` e `.gss` de `views/` |
| `webui.md` | `crates/daemon/webui` (JS) |
| `importer.md` | `crates/importer` |
| `tools.md` | `tools/` (o próprio indexer) |
| `comandos.md` | tabela **Command → handler → quem chama na GUI/webui** |

### Por que um índice separado só de `Command`?

É o eixo que mais atravessa o projeto: uma feature quase sempre é "um
`Command` novo, um handler no daemon e uma chamada na GUI e na webui".
Hoje, para seguir esse fio, é preciso abrir `api/routes.rs`, o handler, o
Luau e o JS. Uma tabela de uma linha por comando (cerca de 170 linhas)
responde de uma vez "quem trata `ServiceStop` e de onde ele é chamado".
Isso também ajuda na regra de que a GUI e a webui são dois clientes: um
comando que aparece só num dos dois lados fica visível na tabela.

## 4. Formato de cada entrada (compacto de propósito)

Cada token do índice é pago toda vez que alguém o lê, então o formato é
enxuto. Exemplo do degrau 2:

```markdown
## deploy/recovery.rs — reconcilia estado do banco com o Docker no boot
fn recover(db, docker, ingress, tls, secrets, bus) — …primeira linha do ///…
fn reconcile_service(…) — …

## docker/containers.rs — ciclo de vida de containers via API Docker
struct ContainerSpec — parâmetros de criação de um container
impl DockerClient: create_container, start, stop, remove, inspect, logs
fn container_name(service) -> String — nome estável rp_<safe_name>
```

Regras:

- **Assinatura resumida**: só os nomes dos parâmetros, sem os tipos (os
  tipos custam muito e raramente ajudam a achar). O retorno aparece só
  quando é informativo (`-> String`, `-> Result<Deploy>`).
- **Métodos agrupados por `impl`** numa linha só. Descrição individual só
  para métodos que têm `///`.
- **Descrição = primeira frase do `///` ou do `//!`**. Nada é inventado:
  se não há doc, a entrada sai só com o nome, que muitas vezes já basta.
- **Itens privados entram**. Muitas responsabilidades moram em `fn`
  privada dentro de arquivo grande; tirá-las derrotaria o objetivo.
- **Testes (`#[cfg(test)]`, `tests/`)** entram só como uma linha por
  arquivo ("testes de render de templates"), sem listar cada `#[test]`.
- **Ficam de fora**: `templates/blueprints` (~780 arquivos de dados; vira
  uma linha só no INDEX), imagens, fontes, `target/`, `dist/`.

## 5. Quem gera o índice: um programa, não uma pessoa (nem um LLM)

O índice precisa ser **gerado automaticamente** a partir do código. Se for
escrito à mão, fica desatualizado em uma semana, como aconteceu com
CLAUDE.md e AGENTS.md. Se for escrito por um LLM, custa tokens toda vez e
pode inventar coisas.

### Rust: parser de verdade (`syn`), não regex

A proposta é um pequeno binário `tools/indexer` (membro do workspace,
`publish = false`) que usa a crate `syn` para ler cada `.rs`. Por que
`syn` e não regex ou ctags:

- **regex** erra em casos reais: `impl<T> Foo<T> for Bar`, `fn` dentro de
  macro, `pub(crate) async unsafe fn`, métodos cujo `impl` fica em outro
  arquivo. O `syn` entende a sintaxe e sabe a que `impl` um método pertence.
- **ctags** (nem está instalado aqui) gera um arquivo pensado para
  editores: grava a posição de cada símbolo, não tem descrição nem
  agrupamento, e é verboso para um LLM ler.
- **rust-analyzer** dá símbolos, mas não gera um documento legível. Ele
  continua útil *junto* (ver seção 7).

O `syn` já é dependência transitiva do projeto (via serde_derive etc.),
então quase não aumenta o tempo de build.

### Luau, JS e `.gv`: regex basta

Esses arquivos são pequenos e têm padrões regulares (`local function x`,
`function M.x`, `function x(`, `const x = (`…). O mesmo binário trata
esses arquivos com regex. Para `.gv`, lista só a casca (`<screen>` ou
`<component>`), os componentes importados e os handlers referenciados
(`onClick="…"`), porque é isso que liga a tela ao Luau.

### A tabela de comandos

O indexer lê `enum Command` em `crates/shared` (as variantes), o `match`
em `crates/daemon/src/api/routes.rs` (variante → handler) e procura o nome
de cada variante nos `.luau` e `.js` (quem chama). Tudo por nome, sem
linha.

## 6. Manter atualizado sem esforço

Como o índice não tem número de linha, ele só muda quando a *estrutura*
muda. Proposta:

- `make index` roda `cargo run -p indexer` e reescreve `docs/indice/`.
- `cargo run -p indexer -- --check` compara com o que está commitado e
  falha se estiver velho. Dá para colocar isso num pre-commit local ou no
  CI. **Fica como opcional**: você prefere configuração mínima (ver o
  histórico do postinst), então começo só com o `make index` e decidimos
  depois se vale automatizar.
- O índice fica **commitado** no git. Assim o diff de um commit mostra
  também "entrou a função X, saiu a Y", o que é um bom resumo estrutural.
- A saída é **determinística** (ordem alfabética de caminho, depois ordem
  de declaração no arquivo), para não gerar diff sem mudança real.

## 7. Como os agentes vão saber que o índice existe

- Uma linha no `AGENTS.md` (Parte 2 — Convenções): *"Para localizar
  código, comece por `docs/indice/INDEX.md`, depois `grep -n` pelo nome.
  Não leia arquivos grandes inteiros."* O CLAUDE.md já aponta para o
  AGENTS.md, então não precisa mexer nele.
- Uma memória minha registrando o fluxo.
- **Complemento**: quando o nome já é conhecido, a ferramenta LSP (o
  rust-analyzer, que está instalado) acha a definição direto, sem índice.
  O índice resolve *responsabilidade → nome*; o grep ou o LSP resolvem
  *nome → linha*.

## 8. Fase opcional: melhorar as descrições

O índice é tão bom quanto os comentários de doc. Hoje 102 dos 157 `.rs`
**não** têm `//!`, então no INDEX.md esses arquivos aparecem sem
descrição. Proposta de uma passada única: escrever uma linha de `//!` em
cada um desses arquivos (e `///` nas funções públicas grandes que não têm
nenhum). É um custo único em tokens e melhora o próprio código, não só o
índice. Pode ser feito por área, aos poucos, e não bloqueia as outras
fases.

## 9. Fora de escopo (e por quê)

- **Busca semântica/embeddings**: exigiria um serviço, um banco de vetores
  e reindexação. É desproporcional para 34 mil linhas; um sumário bem
  feito resolve.
- **Grafo de chamadas** (quem chama quem): é caro de gerar direito (exige
  resolução de tipos, na prática o rust-analyzer) e ficaria grande demais.
  A exceção é a tabela de `Command`, que cobre o eixo mais importante.
- **Indexar `docs/`**: os docs já têm títulos e cabeçalho de status. O
  INDEX.md só lista os nomes, com a primeira linha de cada um.

## 10. Fases de execução

1. ✅ **Indexer Rust** (`tools/indexer` com `syn`): gera `INDEX.md` e os
   índices por área dos crates Rust. Na prática o `daemon-infra.md`
   original deu ~10 mil tokens e foi dividido em quatro áreas; cada área
   ficou entre ~0,5 e ~6 mil tokens (`shared.md` é a maior, puxada por
   `models.rs`). O INDEX.md ficou em ~4 mil, acima da meta, principalmente
   por causa dos títulos de `docs/`.
2. ✅ **Luau/JS/.gv/HTML** (`tools/indexer/src/script.rs`, regex por linha):
   `gui-handlers.md`, `gui-scripts.md`, `gui-views.md` e `webui.md`. Os
   scripts Luau davam ~8 mil tokens juntos e foram divididos em handlers e
   o resto. Do `.gv` saem casca, props, imports, script, views e handlers
   chamados; do `index.html`, as seções `── X ──`, os `x-data` e os métodos
   chamados em `@click`/`@submit`.
3. ✅ **Tabela de comandos** (`tools/indexer/src/commands.rs`): `comandos.md`,
   ~3 mil tokens, agrupada pelos comentários do `enum Command`. Além de GUI e
   webui, ganhou uma coluna `agente:` (a API de agente em
   `crates/rustploy-gui/src/agent/` também manda comandos, o que o plano não
   previa). Termina com uma seção de paridade: o que só a GUI usa, o que só a
   webui usa e o que nenhuma das duas usa.
4. ✅ **Integração**: `make index`; seção "Achar código" no início da Parte 2
   do AGENTS.md, mais a linha do indexer na tabela de crates; uma linha no
   topo da tabela do CLAUDE.md (é o arquivo que o Claude lê em toda sessão);
   e a memória.
5. *(Opcional)* `--check` em pre-commit ou CI.
6. *(Opcional)* Passada de `//!` nos 102 arquivos sem descrição.

Critério de pronto: pegar 5 perguntas reais ("onde o deploy decide que
falhou?", "quem grava a rota de ingress?", "onde a webui mostra toast?"…)
e responder cada uma lendo só o índice mais um grep e um trecho, medindo
os tokens gastos contra o fluxo de hoje.
