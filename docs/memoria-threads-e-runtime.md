# Memória de threads e runtime async — base teórica

Este documento fundamenta uma estimativa feita durante a discussão "sync vs tokio"
no client: *"a stack de 8 MB por thread é memória virtual; o que conta de RSS são
as páginas tocadas (~8–32 KB por thread); um client sync com 2–3 threads
provavelmente usa menos RAM que o runtime multi-thread do tokio"*.

A meta aqui não é só registrar a conclusão, mas o **raciocínio** — e as fontes
para reconstruí-lo sozinho.

---

## 1. Memória virtual ≠ memória física

Todo processo Linux enxerga um espaço de endereçamento virtual próprio. Quando o
processo faz `mmap` de uma região (e a stack de uma thread é exatamente isso),
o kernel **não aloca RAM nenhuma**: ele apenas registra a região na tabela de
mapeamentos do processo. A RAM física só é consumida quando o processo **toca**
uma página (4 KiB) pela primeira vez — o acesso gera um *page fault*, e só então
o kernel mapeia um frame físico ali. Isso se chama *demand paging* (e, para
memória anônima zerada, *demand zeroing*).

Disso seguem as duas métricas que aparecem em `ps`, `top` e `/proc`:

| Métrica | Nome em `/proc/<pid>/status` | O que mede |
|---------|------------------------------|------------|
| VSZ     | `VmSize`                     | Total **mapeado** (promessa, não custo) |
| RSS     | `VmRSS`                      | Páginas **residentes em RAM** (custo real) |

Regra prática: **VSZ assusta e não importa; RSS é o que você paga.** Em Linux
com *overcommit* habilitado (o padrão, `vm.overcommit_memory=0`), o kernel
aceita mapear muito mais memória do que existe fisicamente, justamente porque a
maioria das regiões nunca é tocada por inteiro.

(RSS ainda superestima em um cenário: páginas compartilhadas entre processos
são contadas inteiras em cada um. Para isso existe a PSS em
`/proc/<pid>/smaps_rollup` — para um daemon único, RSS ≈ PSS e não precisamos
nos preocupar.)

## 2. A stack de 8 MiB por thread

Quando você cria uma thread com `pthread_create` (que é o que
`std::thread::spawn` usa por baixo), a glibc faz `mmap` de uma stack cujo
tamanho default vem de `ulimit -s` — tipicamente **8 MiB** em x86-64. Mas, pelo
mecanismo da seção 1:

- Os 8 MiB entram no **VSZ** imediatamente.
- O **RSS** só cresce conforme a thread usa a stack de verdade. Uma thread que
  faz I/O bloqueante e chama meia dúzia de funções toca 2–8 páginas: **8–32 KiB**.
- Uma página extra fica reservada como *guard page* (protegida, sem acesso) no
  fim da stack para transformar estouro de stack em SIGSEGV em vez de corrupção
  silenciosa — ver `pthread_attr_setguardsize(3)`.

Ou seja: 100 threads dormindo = ~800 MiB de VSZ e ~1–3 MiB de RSS. É por isso
que "disparar threads sem tokio" não pesa na RAM como a intuição sugere.

Se ainda assim quiser reduzir o teto (útil em proxies thread-per-connection):

```rust
std::thread::Builder::new()
    .stack_size(128 * 1024) // 128 KiB de teto em vez de 8 MiB
    .spawn(|| { /* ... */ })?;
```

Em Rust há também a env var `RUST_MIN_STACK` para o default global. Atenção: o
limite baixo vale para o *pior caso* da stack (recursão, buffers grandes em
stack como `[u8; 1MB]`), não para o caso típico — dimensione olhando o código
que roda na thread.

### Experimento de 5 minutos para ver isso acontecendo

```rust
fn main() {
    let pid = std::process::id();
    println!("antes: veja /proc/{pid}/status (VmSize, VmRSS)");
    std::thread::sleep(std::time::Duration::from_secs(15));

    let handles: Vec<_> = (0..100)
        .map(|_| std::thread::spawn(|| std::thread::sleep(std::time::Duration::from_secs(60))))
        .collect();

    println!("depois: veja de novo — VmSize ~+800 MiB, VmRSS ~+2 MiB");
    handles.into_iter().for_each(|h| { h.join().unwrap(); });
}
```

Em outro terminal: `grep -E 'VmSize|VmRSS' /proc/<pid>/status`. Havia um script
`measure_ram.sh` na raiz do repo que fazia a versão "produção" disso comparando
`rustployd` com o cliente TUI (`rustploy`) — removido junto com o TUI; a
metodologia abaixo continua válida para medir só o daemon.

## 3. O que o tokio realmente cria

O custo de base do runtime multi-thread do tokio (o que `#[tokio::main]` usa):

- **Worker threads**: uma por core lógico por default
  (`Builder::worker_threads`). Numa máquina de 8 cores são 8 threads — cada uma
  com sua stack de 8 MiB virtuais e seus poucos KiB residentes, exatamente como
  na seção 2.
- **Blocking pool**: threads criadas sob demanda para `spawn_blocking` (o sqlx
  e o `git2` caem aqui), com teto default de **512**
  (`Builder::max_blocking_threads`), recicladas após ociosidade.
- Estruturas do runtime em si: filas de tarefas, driver de I/O (epoll), timer.

Conclusão da comparação: um client **sync com 2–3 threads** tem base de RAM
menor ou igual à de um runtime tokio com N workers — a vantagem do async não é
RAM em baixa escala, é **escala de concorrência** (uma task `async` custa
centenas de bytes a poucos KiB no heap, contra dezenas de KiB residentes + 8 MiB
virtuais de uma thread; com 10.000 conexões simultâneas a conta inverte
completamente). Para um cliente com poucas conexões (ex.: 2 UDS), threads
ganham ou empatam.

## 4. Como medir, em ordem de fidelidade

1. `grep -E 'VmSize|VmRSS|VmHWM|Threads' /proc/<pid>/status` — snapshot rápido;
   `VmHWM` é o pico histórico de RSS.
2. `cat /proc/<pid>/smaps_rollup` — RSS/PSS consolidados; `smaps` (sem rollup)
   mostra região por região, dá para ver cada stack de thread individualmente.
3. `ps -o pid,vsz,rss,nlwp,comm -p <pid>` — inclui contagem de threads (`nlwp`).
4. `ps -o pid,vsz,rss,nlwp,comm -p $(pgrep rustployd)` num loop (`watch`) —
   monitora o daemon deste projeto ao longo do tempo.

---

## Referências

### Livros (teoria de base)

- **OSTEP — Operating Systems: Three Easy Pieces**, Remzi & Andrea
  Arpaci-Dusseau. Gratuito em <https://pages.cs.wisc.edu/~remzi/OSTEP/>.
  *A* referência para adquirir este raciocínio. Os capítulos 13 (Address
  Spaces), 15–16 (Address Translation/Segmentation), 18–20 (Paging) e 21–23
  (Beyond Physical Memory — onde entram demand paging e lazy allocation)
  cobrem toda a seção 1 deste documento. Didático, curto por capítulo.
- **The Linux Programming Interface**, Michael Kerrisk (No Starch Press, 2010).
  A visão "como o Linux faz de verdade": layout de memória do processo
  (cap. 6–7), threads POSIX e suas stacks (caps. 29–33), `mmap` (cap. 49),
  limites de recursos/`RLIMIT_STACK` (cap. 36). Kerrisk é o mantenedor das man
  pages do Linux, então o livro conversa direto com as referências abaixo.
- **Systems Performance, 2ª ed.**, Brendan Gregg (Addison-Wesley, 2020).
  Cap. 7 (Memory) explica VSZ vs RSS vs PSS, demand paging, overcommit e as
  ferramentas de medição — é o capítulo que transforma a teoria do OSTEP em
  prática de observação. Material complementar em
  <https://www.brendangregg.com/>.

### Papers e documentação do kernel

- **"What Every Programmer Should Know About Memory"**, Ulrich Drepper, 2007.
  PDF: <https://people.freebsd.org/~lstewart/articles/cpumemory.pdf> (também
  publicado em série no LWN: <https://lwn.net/Articles/250967/>). Vai além do
  necessário aqui (caches, NUMA), mas as seções iniciais sobre memória virtual
  são canônicas.
- **Overcommit accounting** (documentação do kernel):
  <https://www.kernel.org/doc/html/latest/mm/overcommit-accounting.html> —
  explica por que o kernel deixa o VSZ passar (muito) da RAM física.

### Man pages (a fonte primária — leia estas primeiro)

- `man 5 proc` — significado de `VmSize`, `VmRSS`, `VmHWM`, `smaps`.
- `man 3 pthread_create` e `man 3 pthread_attr_setstacksize` — documentam o
  default de 8 MiB vindo de `RLIMIT_STACK`/`ulimit -s`.
- `man 3 pthread_attr_setguardsize` — a guard page da stack.
- `man 2 mmap` — `MAP_ANONYMOUS`, `MAP_STACK`, `MAP_NORESERVE`; é aqui que
  "mapear não é alocar" está escrito preto no branco.
- `man 2 getrlimit` — `RLIMIT_STACK`.

### Rust e tokio

- `std::thread` — docs do módulo explicam stack size, `Builder::stack_size` e
  `RUST_MIN_STACK`: <https://doc.rust-lang.org/std/thread/>.
- `tokio::runtime::Builder` — defaults de `worker_threads` (nº de cores) e
  `max_blocking_threads` (512):
  <https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html>.
- `tokio::task::spawn_blocking` — quando o tokio cria threads extra:
  <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>.
- **The Rust Performance Book**, Nicholas Nethercote — capítulo de heap
  profiling para medir consumo de programas Rust:
  <https://nnethercote.github.io/perf-book/>.

### Roteiro de estudo sugerido

1. OSTEP caps. 13, 18 e 21 (uma tarde) → entende *demand paging*.
2. `man 5 proc` + experimento da seção 2 (meia hora) → vê VSZ vs RSS ao vivo.
3. Gregg cap. 7 → consolida o vocabulário de medição.
4. TLPI caps. 29–33 quando for escrever código com threads de verdade.
