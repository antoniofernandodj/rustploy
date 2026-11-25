# Estudo: consumo de RAM (daemon, GUI e glacier-ui)

> **Arquivado em 2026-11-25** (revisão das docs): concluído ou substituído. O que ainda vale fazer foi para o
> `TODO.md`; o que não fazia mais sentido saiu daqui.

> **Estado:** implementado e publicado em 2026-11-20 — glacier-ui 0.168.1, daemon 0.1.68 e app 0.1.76
> (release v0.1.82). Ver a seção 7, "O que foi feito e o que a medição mostrou". O texto das seções 1–6 é o estudo
> original, mantido como estava. Complementa o
> [`estudo-seguranca-e-desempenho.md`](../estudo-seguranca-e-desempenho.md) (2026-11-10), que já
> listava os itens de CPU/rede (P-1…P-10); aqui o foco é **memória**, com números medidos nesta
> máquina. Onde algo é estimativa, está dito.

## 1. Onde estamos (medido)

Medido em `/proc/<pid>/smaps_rollup` com os binários instalados (GUI 0.1.73, daemon 0.1.65),
app aberto há ~1h30, Intel HD 2500 com `WGPU_BACKEND=gl`.

| Processo | RSS | **PSS** (o custo justo) | Pico (`VmHWM`) | Threads |
|---|---|---|---|---|
| `rustploy-gui` | 135 MB | **64 MB** | 145 MB | 16 |
| `rustployd` | 50 MB | (sem permissão p/ ler) | 65 MB | 12 |

**Por que olhar o PSS e não o RSS.** O RSS conta inteira cada biblioteca compartilhada que o
processo tocou. A GUI aparece com 47 MB de `libLLVM` (o driver OpenGL do Mesa carrega o LLVM),
mas esse arquivo está mapeado por outros 11 processos do desktop — a parte que cabe à GUI é 5 MB.
Por isso trocar o renderizador para software (tiny-skia) **não** economizaria os ~55 MB que o RSS
sugere; no máximo uns 7 MB reais. Não vale como primeira medida.

Decomposição da GUI (PSS):

| Parte | MB | O que é |
|---|---|---|
| binário `/usr/bin/rustploy-gui` | 23,5 | código + views/scripts/ícones embutidos (páginas de arquivo, o SO pode descartar sob pressão) |
| `[heap]` | 19,2 | malloc do glibc: árvore de widgets, contexto, caches, **VMs Luau** |
| anônima sem nome | 7,5 | arenas extras do malloc (uma por thread), buffers do wgpu/GL |
| LLVM + gallium + GTK | ~7,5 | driver gráfico e bandeja, divididos com o desktop |

Ou seja: **o que dá para reduzir de verdade na GUI são os ~27 MB de heap/anônima** (sujos, o SO
não pode descartar). No daemon, ~50 MB para um processo com tokio, SQLite, proxy e cliente Docker
é razoável; o que pesa lá não é o repouso, são os **picos** (o `VmHWM` de 65 MB) e o que ele faz a
cada 2 s.

## 2. A causa principal, dos dois lados: o snapshot de 2 s

Já era o P-1 do estudo anterior (prioridade 3) e **continua igual**
(`api/http/mod.rs::events`). Do ponto de vista de memória ele é o maior desperdício porque gera
lixo grande e repetido nos dois processos:

**No daemon**, por conexão SSE (GUI + cada aba da webui), a cada 2 s: ~15 `dispatch`, uma
`ServiceList` por projeto, listas do Docker, do registry, auditoria de infra… tudo montado num
`serde_json::Value` (cada objeto JSON vira um `Map` alocado, cada string uma `String`) e depois
num `String` final. Com 3 clientes são 3 cópias simultâneas da mesma coisa. O `mimalloc` devolve
essa memória, mas o **pico** sobe com o número de clientes.

**Na GUI** o mesmo texto passa por **até cinco cópias**, todas vivas ao mesmo tempo:

1. `json.decode` do snapshot inteiro em tabelas Luau (`handlers/stream.luau::apply_snapshot`);
2. o `apply_snapshot` grava listas formatadas no `ctx` como **tabelas** (`ctx.ingress`,
   `ctx.monitoring`, `ctx.deploy_engine_*`, `ctx.jobs_summary`, as de `rebuild_lists`…);
3. no fim do handler, `sync_from_luau` (glacier-ui `src/luau/mod.rs`) **re-serializa cada tabela
   para texto JSON** (`table_to_json`) só para comparar com o que já estava — e guarda o texto
   duas vezes: em `ctx.data` e de novo no `json.cache` (`cache.insert(k, (s.clone(), t))`);
4. no começo do próximo handler, `sync_to_luau` **clona o texto de toda chave JSON** para
   `json.pendentes` (`json.pendentes.insert(k.clone(), v.clone())`) — mesmo que o script nem a toque;
5. o `EvalCache` do render guarda **mais uma cópia do texto** de cada lista do `for-each`, mais a
   lista já parseada (`serde_json::Value`).

Nada disso é vazamento (tudo é liberado), mas define o **patamar** do heap: o malloc do glibc
raramente devolve memória ao SO depois de um pico, e o GC do Luau só coleta quando o heap Luau
cresce ~2× sobre o que está vivo (meta padrão de 200%).

## 3. O que fazer, na ordem em que eu atacaria

Gravidade aqui = quanto de RAM se espera ganhar. Esforço: P (horas), M (dias), G (semana+).

### R-1 · Snapshot compartilhado e só com o que mudou — Alto · M (daemon + GUI + webui)

É o P-1 do estudo anterior, itens 1–3. Resumo do porquê pensando em RAM:

- **Uma** task calcula o snapshot e publica num `watch<Arc<str>>`; cada conexão SSE só repassa o
  mesmo `Arc` — some a cópia por cliente, e nada é calculado se não houver cliente.
- Enviar **por seção** e só as seções que mudaram (hash por seção). No caso comum (nada mudou) a
  GUI não decodifica nada, não refaz as listas e não re-serializa — as cópias 1–5 da seção 2
  simplesmente não acontecem a cada 2 s.
- Tirar do tick o que é estático (`docker_images/volumes/networks`, `registry_repos`,
  `infra_audit`, `state_git`): buscar quando a tela abre ou quando um evento avisar.

Exige mexer nos **dois clientes** (GUI e webui) — o contrato do evento `snapshot` muda.

### R-2 · glacier-ui: parar de copiar o texto JSON a cada handler — Alto · M (glacier-ui)

Independe do R-1 e beneficia qualquer app glacier com listas grandes no contexto.

- **Valores do contexto em `Arc<str>`** (ou `Rc<str>`, a UI é uma thread só) em vez de `String`.
  Aí `pendentes`, `json.cache` e o `EvalCache` passam a guardar **o mesmo ponteiro**, não cópias;
  e a validação do `EvalCache` ganha um caminho rápido (`Rc::ptr_eq`) antes do `memcmp`.
  É a mudança mais invasiva (o tipo do `ContextMap` aparece em muitos lugares), mas mecânica.
- Sem mudar o tipo, dá para cortar duas cópias já:
  - `sync_to_luau`: `pendentes` guardar **só a chave**; o `__index` preguiçoso busca o texto no
    contexto quando o script acessa. (Hoje cada turno clona o texto de todas as chaves JSON.)
  - `sync_from_luau`: o `json.cache` guardar um **hash** do texto (ou nada, comparando com
    `ctx.data`) em vez do `s.clone()`.
- **Não re-serializar tabela que o script não alterou.** Hoje toda chave JSON materializada é
  recodificada no fim do turno. Uma tabela-proxy com `__newindex` que marca "suja" (só nas chaves
  de topo já ajudaria) ou, mais simples, comparar a identidade da tabela: se o script atribuiu
  `ctx.x = outra_tabela`, re-serializa; se só leu `ctx.x`, não. *Cuidado:* mutação interna
  (`ctx.lista[3].nome = …`) também tem de contar — precisa de teste antes de confiar.

### R-3 · Medir a memória do Luau e expor no `GLACIER_PERF` — Médio · P (glacier-ui)

Hoje não há como saber quanto do heap de 19 MB é Luau. `mlua` dá `Lua::used_memory()`: imprimir
por motor (janela) junto do que o `GLACIER_PERF=1` já mostra. Sem isso, R-4 e R-5 são chute.

### R-4 · Ajuste do GC do Luau — Médio · P (glacier-ui)

Luau usa GC incremental com meta de 200% (o heap pode chegar a ~2× o vivo antes de coletar). O
snapshot gera muito lixo de vida curta. Duas alavancas, a medir com R-3:
- `lua.gc_inc(goal, step_multiplier, step_size)` com meta menor (ex.: 150) — menos pico, mais CPU;
- um `gc_collect()`/passo forçado logo **depois** de um handler que trocou muito dado (o motor sabe
  quando um handler de `sse` terminou). Expor como opção do builder, padrão desligado.

O `set_memory_limit` (G-2 do estudo anterior) entra junto: é segurança contra crescimento sem fim.

### R-5 · Uma VM Luau por janela — Médio · M (glacier-ui)

`build_with` faz `Lua::new()`, compila o prelúdio (66 KB de fonte) e instala todos os globais
**por janela/componente com script**; cada `require` recompila o módulo dentro da VM. A janela de
logs, o "Novo job", as de projeto/serviço… cada uma tem uma cópia própria do bytecode dos módulos
que puxa. Ganhos possíveis, a confirmar com R-3:
- compilar prelúdio e módulos **uma vez** para bytecode (`mlua::Compiler`) e reaproveitar o
  bytecode entre VMs (menos CPU ao abrir janela; o bytecode carregado ainda é por VM);
- fechar de fato a VM quando a janela fecha (conferir se nada a segura — `Rc` em closures de
  timers/streams).

### R-6 · Alocador da GUI — Médio · P (rustploy-gui; só medir e trocar)

A GUI usa o malloc do glibc com 16 threads. O glibc cria até 8×núcleos **arenas** (uma por thread
que aloca), e memória livre numa arena não serve às outras — é a "anônima sem nome" de 7,5 MB e
parte do heap. Duas experiências baratas, comparando o PSS depois de 10 min com a tela de serviços
aberta:
1. rodar com `MALLOC_ARENA_MAX=2` (sem recompilar);
2. `mimalloc` como `#[global_allocator]` no `main.rs` da GUI (o daemon já usa).

Se um dos dois ganhar, fica. Custo zero de manutenção.

### R-7 · Coleta de métricas carrega todos os processos do host — Baixo · P (daemon)

`features/observability/metrics.rs::collect_loop` cria `System::new_all()`: isso lê **todos os
processos do servidor** (nome, cmdline, ambiente, exe…) e os guarda para sempre, mas o laço só usa
CPU e memória (`refresh_cpu_usage`/`refresh_memory`). Trocar por `System::new()` (ou
`new_with_specifics(RefreshKind::nothing().with_cpu(..).with_memory(..))`). Num servidor com
centenas de processos são alguns MB parados. Junto, e pelo mesmo motivo:
- o laço roda a cada 2 s **mesmo sem nenhum cliente** (e faz um `docker stats` por serviço): só
  coletar se `bus.receiver_count() > 0`, ou subir o intervalo quando ninguém olha (P-6);
- `Disks::new_with_refreshed_list()` é recriado a cada tick: guardar e só dar `refresh`.

### R-8 · Dependências do daemon (binário menor, menos páginas de código) — Baixo · P

Ainda pendentes do P-4: `sqlx` com `runtime-tokio-native-tls` (puxa OpenSSL só para SQLite →
`runtime-tokio`), `rcgen` com `aws_lc_rs` ao lado do `ring` (→ feature `ring`), `tokio`/`hyper`
com `full`, `reqwest` só para o `gitea.rs`. O ganho em RAM é pequeno (código é página de arquivo,
descartável), o ganho real é build e binário. Fica aqui porque soma.

### R-9 · Imagens embutidas copiadas para o heap — Baixo · P (glacier-ui)

`cached_image_handle`/`cached_svg_handle` (`src/widget.rs`) fazem
`read_bytes(..).into_owned()`: no release os bytes já estão no binário (`Cow::Borrowed`), e o
`into_owned` copia cada ícone/logo usado para o heap. Com `Cow::Borrowed(b)` usar
`Bytes::from_static(b)` (o `image::Handle::from_bytes` aceita `Into<Bytes>`). Ganho = soma dos
ícones exibidos (centenas de KB a poucos MB, dependendo de quantos logos a tela de templates
mostrou). O RGBA decodificado no cache do iced não muda com isso.

## 4. O que eu **não** faria

- **Renderizador de software para economizar RAM**: ver seção 1 — o LLVM é compartilhado; o ganho
  real é ~7 MB e o custo é CPU por quadro.
- **`panic = "abort"` no daemon**: o comentário do `Cargo.toml` do daemon ainda fala do perfil
  dividido com a GUI (não é mais, são repos separados), então tecnicamente dá. Mas hoje um `panic`
  numa task do tokio mata só aquela task; com `abort` derruba o daemon inteiro. Binário uns 10%
  menor não paga isso. (Vale só corrigir o comentário.)
- **Menos threads do tokio no daemon**: 12 threads custam pouco de RSS (a pilha de 2 MiB é
  virtual; ver [`memoria-threads-e-runtime.md`](memoria-threads-e-runtime.md)).

## 5. Como medir antes e depois

```bash
# PSS da GUI (o número a comparar) e o pico
grep -E '^(Pss|Private_Dirty):' /proc/$(pgrep -x rustploy-gui)/smaps_rollup
grep VmHWM /proc/$(pgrep -x rustploy-gui)/status
# daemon (precisa de sudo: roda como outro usuário)
sudo grep -E '^(Pss|Private_Dirty):' /proc/$(pgrep -x rustployd)/smaps_rollup
# R-6 sem recompilar
MALLOC_ARENA_MAX=2 rustploy-gui
```

Cenário fixo para comparar: GUI aberta 10 min na tela de serviços, conectada ao mesmo servidor,
webui fechada; depois o mesmo com 2 abas da webui abertas (mede o efeito por cliente do R-1).
Ainda falta medir **o tamanho em bytes de um snapshot** num servidor real (ex.: Chiquitos) — é o
número que diz quanto o R-1 e o R-2 valem.

## 6. Resumo da ordem

| # | Item | Onde | Ganho esperado | Esforço |
|---|---|---|---|---|
| 1 | R-3 medir Luau no `GLACIER_PERF` | glacier-ui | dá os números para o resto | P |
| 2 | R-6 `MALLOC_ARENA_MAX`/mimalloc na GUI | gui | alguns MB, de graça | P |
| 3 | R-7 métricas sem `new_all` e só com cliente | daemon | alguns MB + CPU ociosa | P |
| 4 | R-2 cortar cópias de JSON no contexto | glacier-ui | patamar do heap da GUI | M |
| 5 | R-1 snapshot compartilhado e por seção | daemon+gui+webui | maior ganho geral (RAM, CPU, rede) | M |
| 6 | R-4 GC do Luau, R-5 VM por janela | glacier-ui | depende do R-3 | P/M |
| 7 | R-8, R-9 | daemon / glacier-ui | pequeno | P |

## 7. O que foi feito e o que a medição mostrou (2026-11-20)

### Bancada

Duas cópias do `rustploy-gui` no scratchpad (nenhum repo tocado), compiladas em release sem LTO:
`base` = glacier-ui 0.167 do crates.io; `novo` = o branch `perf/ram` do glacier-ui. Rodaram lado a
lado num Xvfb, conectadas ao daemon local (4 serviços), 200 s cada, medindo `smaps_rollup`. No Xvfb
não há GPU (o wgpu cai no llvmpipe), então os **absolutos** são maiores que no desktop (~140 MB de
PSS); vale só a **comparação** entre as variantes.

| Variante | Anônima | PSS |
|---|---|---|
| base | 57,3 MB | 138,7 MB |
| base + `MALLOC_ARENA_MAX=2` | **52,8 MB** | **135,1 MB** |
| base + jemalloc (`LD_PRELOAD`) | 62,4 MB | 145,2 MB |
| novo (R-2/R-3/R-9) | igual à base (±1 MB) | igual à base |

E a linha nova do `GLACIER_PERF` disse o que faltava saber: **a VM Luau da janela principal ocupa
3,1 MB**. O heap da GUI (~36 MB no Xvfb, ~19 MB no desktop) é quase todo iced/wgpu/cosmic-text, não
Luau nem o contexto.

### Feito

| Item | Onde | Situação |
|---|---|---|
| R-1 snapshot compartilhado e por seção | daemon `api/http/snapshot.rs` (novo), `mod.rs`, `state.rs`; GUI `handlers/stream.luau`; webui `app.js` | feito, testado de ponta a ponta |
| R-2 sem cópias de JSON por turno | glacier-ui `src/luau/mod.rs` (`JsonState` → espelho por chave) | feito; 452 testes da lib ok |
| R-3 memória Luau no `GLACIER_PERF` | glacier-ui `src/perf.rs` (linha `memória:` com RSS e heap por VM) | feito |
| R-6 arenas do malloc | GUI `src/main.rs` (`mallopt(M_ARENA_MAX, 2)`) | feito; **−4,5 MB** medidos |
| R-7 métricas | daemon `metrics.rs` (sem `new_all`, discos reaproveitados, pula sem cliente) e `event_bus.rs` (`receiver_count`) | feito |
| R-8 dependências | daemon: `sqlx` com `runtime-tokio` — **OpenSSL saiu** do daemon (`cargo tree -i native-tls` vazio) | parcial (ver abaixo) |
| R-9 imagens embutidas | glacier-ui `src/widget.rs` (`Bytes`/`Cow` emprestado, sem cópia) | feito |
| comentário do `panic = "abort"` | daemon `Cargo.toml` | atualizado (fica `unwind`, com o motivo novo) |

**R-1 em números** (daemon de teste, 1 projeto): o snapshot inteiro tem **12 KB**; com `?patch=1`
cada tick manda **118–446 bytes** (só `status` e `engine`, que mudam a todo tick por causa do
uptime). Criar um projeto gerou um patch com `projects`; a GUI mostrou "Projects (2)" e o uptime
seguiu andando pelo caminho leve (sem refazer as listas). Cliente antigo (sem o parâmetro) segue
recebendo o snapshot inteiro; daemon antigo ignora o parâmetro e a GUI/webui novas tratam o
`snapshot` como antes. O cálculo agora é **um por tick para todos os clientes**, e nenhum sem cliente.

Testes: daemon 700 ok (inclui a webui no Chrome headless e 3 testes novos do patch); GUI 471 testes
Luau ok e `luau-lsp analyze` limpo no `stream.luau`; glacier-ui 452 (lib) ok.

### Não feito, e por quê

- **R-2 não mudou o número nesta máquina**: o snapshot do daemon local é pequeno. O ganho aparece em
  servidor com muitos serviços (mais KB por chave JSON copiados a cada turno). Mantido porque é
  correto, testado e não custa nada.
- **R-4 (GC do Luau) e R-5 (VM por janela)**: com a VM principal em 3 MB, ajustar o GC ou dividir
  bytecode economizaria centenas de KB. Não vale a complexidade. Reabrir se o `GLACIER_PERF`
  mostrar uma VM grande num servidor real.
- **Trocar o alocador da GUI**: jemalloc foi pior (+5 MB); o mimalloc é da mesma família. Fica o
  glibc com 2 arenas.
- **R-8, o resto**: o `aws-lc-rs` não sai só trocando a feature do `rcgen` — o `rcgen` gera chave
  **RSA** para o ACME (o `ring` não gera RSA) e o `rustls` de outras dependências (`reqwest`,
  `instant-acme`) também o puxa. Tirar exige ECDSA no ACME e alinhar as features do `rustls` em
  três crates; ganho de RAM pequeno (código é página de arquivo). `tokio`/`hyper` com `full`: idem.

### Publicado

glacier-ui 0.168.1 (com a documentação do `inputgroup`: `prefix`/`suffix` são rótulo, não valor) e
glacier-cli 0.28.20; daemon 0.1.68 e app 0.1.76 na release v0.1.82 (ver `docs/release.md`).
