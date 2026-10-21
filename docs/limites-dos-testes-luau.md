# Limites dos testes Luau do rustploy-gui

> Escrito em 2026-10-21, ao entrar a suíte de `rustploy-gui/tests/` (340 testes, 328 deles novos)
> e o `tela:missing()` (glacier-ui 0.127). Lista o que esses testes **não** provam, para ninguém
> ler "verde" como mais do que ele é. Como escrever e rodar os testes: seção "Build & Run" do
> [`AGENTS.md`](../AGENTS.md).

## 1. O snapshot do daemon não entra nos testes

O snapshot chega por SSE e é aplicado por `apply_snapshot`, uma função **local** de
`handlers/stream.luau` — só roda dentro do callback do stream. Um teste não consegue alimentá-la.

Consequência em `tests/screens_missing.test.luau`: as chaves que cada visão do shell espera do
snapshot (`running_count`, `host_cpu`, `docker_containers_count`…) ficam **congeladas** por visão.
O teste prova que o *markup depende* delas; **não** prova que o stream as escreve com o mesmo
nome. Se o `stream.luau` passar a escrever `ctx.runing_count`, nada aqui acusa — a tela fica vazia.

O que o teste ainda pega: uma chave **nova** lida pelo markup que ninguém publica, e uma esperada
que sumiu (passou a ser publicada no `init`, ou deixou de ser lida).

**O que levantaria o limite:** exportar `apply_snapshot` (por exemplo `M.apply_snapshot` no
`stream.luau`) e chamá-lo com um snapshot de fixture. Aí o `missing()` rodaria *depois* de os dados
chegarem, e as listas congeladas encolheriam para o que de fato é opcional.

## 2. `forget_server` e o sucesso do `connect` não têm teste

Os dois chamam `storage.set("saved_servers", …)`, que grava a lista de servidores salvos **da
máquina de quem roda** (`~/.local/share/rustploy/.glacier-storage/app.json`). Um teste não pode
apagar nem alterar essa lista. O `init` também **lê** esse `storage` e pré-preenche o
`access_key` com o servidor mais usado; por isso todo teste que roda o `init` semeia um
`access_key` próprio, senão o resultado dependeria de quem executa.

O que está coberto: tudo em `connect()` que falha **antes** da rede (token vazio, token da API no
lugar do de conexão, chave corrompida), `access_key_input`, `choose_saved_server`, `init` e
`disconnect` (que só lê o `storage`).

**O que levantaria o limite:** um `storage` isolado em modo de teste (diretório temporário), ou
uma injeção de dependência no `connection.luau`. Nenhum dos dois existe hoje.

## 3. `missing()` lê `erro_<campo>` de todo campo de `<form>`

O `tela:missing()` do motor ignora o `erro_<campo>` dos campos que têm `rules`/`pattern` (o motor
os publica ao enviar), mas **não** o dos campos de `<form>` *sem* regras: o motor lê essa chave por
dentro, e ela aparece como ausente. No app isso só atinge `new_project_form.gvb` (a descrição é
opcional), e o teste nomeia `erro_new_project_description` como ausência esperada, com comentário.

É uma imperfeição do `missing()`, não do app. **O ajuste certo** é no motor
(`glacier-ui` 0.127.1): descartar `erro_<form_control>` para todo `form_control`, não só para os
que têm regras. Até lá a exceção fica nomeada no teste.

## 4. Os testes não são type-checados

O `.luaurc` do `rustploy-gui` é `strict` e o CI só roda `glacier test` — **não** roda
`luau-lsp analyze` sobre `tests/`. Os testes que já existiam (`running.test.luau`) têm erros de
tipo equivalentes no `luau-lsp` (auxiliares com parâmetro opcional, `Argument count mismatch`).

Os 16 arquivos novos começam com `--!nonstrict` e passam limpos no `luau-lsp`; isso é convenção,
não garantia — nada a impõe. Um arquivo novo sem o cabeçalho enche o editor de avisos sem quebrar
o CI.

## O que nenhum teste Luau daqui cobre

Não é um limite desta rodada, mas vale ter junto:

- **Layout.** Largura, `fill` dentro de `shrink`, scroll: o `missing()` e os testes de handler não
  olham o desenho. Só abrir a janela mostra (`make run`, ou o Xvfb descrito no `AGENTS.md`).
- **Rede.** `fetch`, `confirm` e os diálogos suspendem o handler e o teste não os resolve; os
  caminhos de sucesso e de erro de RPC ficam de fora.
- **Lógica em Rust.** Fica nos `#[test]` do `rustploy-gui` (`cargo test --bins`,
  `--test templates_render`).

## Candidatos a investigar (não confirmados)

O teste de `new_job_pick_project` achou uma lista vazia saindo como `"{}"` em vez de `"[]"`
(`json.encode` de tabela vazia). Foi corrigido. Outros pontos montam a lista do mesmo jeito e
**podem** ter o mesmo problema quando ela vem vazia — não foram verificados:

- `handlers/services.luau`: `service_shared_database_projects` (linha ~411) e
  `migration_destinations` (~545), `migration_steps` (~558);
- `handlers/jobs.luau`: `new_job_projects` e `new_job_services` (~152);
- `handlers/bundle_import.luau`: `import_providers` (~129).

Para cada um: o `for-each` do template aceita `"{}"`? Se não, trocar por `json.array(...)`.
