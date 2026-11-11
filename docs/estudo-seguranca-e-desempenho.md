# Estudo: segurança, desempenho e economia de recursos (daemon, GUI e glacier-ui)

> **Estado:** estudo (2026-11-10). **Nada aqui foi implementado.** Feito por leitura de código —
> não houve profiling, medição de binário nem teste de carga. Onde algo não foi verificado, está dito.
> Escopo: `rustploy-daemon`, `rustploy-gui`, `rustploy-shared` (de relance) e `../glacier-ui`.
> Não li inteiro: `dispatch` e os handlers de `Command`, `state_git`, `backup`, `databases`,
> `update/apply.rs` (só a parte de assinatura), `parser.rs`/`widget.rs` do glacier-ui.

Cada item tem **gravidade** (Alta/Média/Baixa), **esforço** (P/M/G) e o **porquê**.
A ordem dentro de cada seção é a ordem em que eu atacaria.

---

## Parte 1 — Segurança

### Modelo de ameaça em uma linha

O daemon roda como usuário `rustploy` **no grupo `docker`**, o que equivale a root no host. Portanto
o que protege o servidor é, na prática: (1) quem consegue falar com a API, (2) o que um `admin` pode
mandar o Docker fazer. O hardening abaixo reduz a superfície, mas não muda isso — só o modo
rootless do Podman muda (ver S-12).

### Daemon

**S-1 · Token do git vaza no log e no argv — Alta · P**
`features/deploy/git.rs`: o clone monta `effective_url` com `https://user:TOKEN@host/…` e faz
`info!(args = %msg)` com o `Debug` do array de argumentos — **o token vai para o journal em texto
claro** a cada clone (a função `redact_url` existe e é usada só na linha de "concluído"). Além disso o
token fica no `argv` do `git` (legível em `/proc/<pid>/cmdline` por qualquer usuário local) e no
`.git/config` do diretório de trabalho (o tar do contexto de build já pula `.git`, em
`engine/images.rs`, então não entra na imagem).
*Correção:* passar a credencial por ambiente, não pela URL — `GIT_CONFIG_COUNT/KEY_0/VALUE_0` com
`http.extraHeader=Authorization: Basic …` (ou um `GIT_ASKPASS` que lê de uma variável); logar só
`redact_url`. Acrescentar `GIT_ALLOW_PROTOCOL=https:ssh` e `GIT_CONFIG_NOSYSTEM=1` ao processo.
Conferir também se a cauda de stderr (`tail`, que vira mensagem de erro de deploy) pode ecoar a URL.

**S-2 · Nenhum limite de tentativas de autenticação — Média/Alta · P**
`POST /api/login`, o Bearer de `/api/*` e o Basic do registry não têm rate limit, atraso nem bloqueio
por IP. O token é de 32 bytes de `/dev/urandom` (`generate_master_token`; não conferi o gerador do
token comum), então força bruta é inviável; o risco real é **custo** (cada tentativa é uma conexão +
log) e ruído. *Correção:* contador de falhas por IP com backoff exponencial em memória, e uma linha de
log estável (`auth_fail ip=…`) que o fail2ban consiga ler. O `warn!` atual também registra o
**comprimento esperado** do token (`auth_check`): tirar.

**S-3 · Nenhum timeout nem teto de conexões em nenhum listener — Alta · M**
API (`api/http/mod.rs`), ingress HTTP e HTTPS (`ingress/proxy.rs`) e port proxy: `http1::Builder::new()`
puro, sem `header_read_timeout` (nem `timer`), sem timeout de handshake TLS, sem semáforo de
conexões, sem timeout de conexão/resposta ao backend, sem idle timeout no `copy_bidirectional` do port
proxy. As portas 80/443 são públicas: um cliente que abre milhares de conexões e manda um byte por
minuto (slowloris) esgota descritores e tasks. *Correção:* `.timer(TokioTimer::new())` +
`.header_read_timeout(10s)`, `tokio::time::timeout` em volta do `accept` TLS (5–10 s), `Semaphore` por
listener, `timeout` no `connect` e no `send_request` do backend (com exceção de upgrade/SSE), e
`tokio::time::timeout` por leitura no port proxy.

**S-4 · Corpos lidos inteiros, sem teto, antes de validar — Média/Alta · P**
`login` (**antes de autenticar**), `mcp`, `rpc` e o upload de zip fazem `body.collect().await` sem
limite; o upload só confere os 100 MiB **depois** de ter lido tudo. O webhook também (depois do
token). *Correção:* `http_body_util::Limited::new(body, N)` (login: 4 KiB; rpc/mcp: poucos MiB; zip:
`MAX_ZIP_BYTES`) e checar `Content-Length` logo no começo.

**S-5 · Upload de zip sem limite descompactado — Média · P**
`extract_zip` valida o caminho (bom: recusa `..`/absoluto) mas não limita tamanho descompactado, nº de
entradas nem razão de compressão: um zip de 100 MiB pode virar dezenas de GiB no disco do host. Roda
síncrono (`std::fs`) — conferir se está dentro de `spawn_blocking`. *Correção:* somar `file.size()` e
abortar acima de um teto (ex.: 2 GiB / 100 mil entradas); rejeitar entradas com `unix_mode` de symlink.

**S-6 · Ingress: cabeçalhos de proxy incompletos — Média · P**
`forward` sobrescreve `X-Forwarded-Proto` (certo) mas **não trata `X-Forwarded-For`/`X-Real-IP`**: o
cliente forja o IP que o app atrás do proxy enxerga, e o app nunca vê o IP real. Também não remove
cabeçalhos hop-by-hop (`Connection`, `Keep-Alive`, `TE`, `Upgrade`…). O redirect HTTP→HTTPS monta
`https://{Host}{path}` com o `Host` do cliente sem validar contra a tabela de rotas (open-redirect por
`Host` forjado; só redireciona se o host for conhecido). *Correção:* definir `X-Forwarded-For` com o
`peer` (anexando, se confiar numa camada acima), `X-Real-IP`, limpar hop-by-hop, e só redirecionar
hosts presentes em `routes`.

**S-7 · Web UI sem CSP nem cabeçalhos de segurança — Média · M**
`api/web_ui` serve JS/HTML sem `Content-Security-Policy`, `X-Frame-Options`/`frame-ancestors`,
`X-Content-Type-Options`, `Referrer-Policy`. O token da API vive no JS da página; **um XSS (ou
clickjacking) = controle total do servidor**. O `index.html` tem 238 KB com Alpine; o build padrão do
Alpine exige `unsafe-eval`, mas existe o `@alpinejs/csp`. *Correção mínima (barata):*
`frame-ancestors 'none'`, `nosniff`, `Referrer-Policy: no-referrer`, e HSTS quando houver
`api.domain`. *Correção completa:* CSP sem `unsafe-eval` migrando para o build CSP do Alpine. Auditar os
usos de `innerHTML` (só achei `icons.js`, que injeta SVG próprio).

**S-8 · Containers sem hardening por padrão — Média · M**
`engine/containers/lifecycle.rs`: o `HostConfig` só define rede, mounts, memória e `cpu_shares`. Sem
`cap_drop: ALL`, sem `no-new-privileges`, sem `pids_limit`, sem limite de log do driver
(`json-file max-size/max-file` — **o disco enche**), memória/CPU ilimitadas por padrão, `userns`
desligado. *Correção:* defaults seguros no `HostConfig` (`cap_drop=["ALL"]` + `cap_add` só do que o
produto precisa; `security_opt=["no-new-privileges"]`; `pids_limit`; `log_config` com rotação), com
opt-out por serviço (bancos e brokers do catálogo precisam de ajuste). Para `volumes` com
`host_path` arbitrário: validar contra uma lista de proibidos (`/var/run/docker.sock`, `/etc`,
`/var/lib/rustploy`, `/root`, `/proc`, `/sys`, `/`) mesmo para `admin`. Stacks Compose passam direto
pelo CLI: o mesmo filtro não alcança — documentar como risco aceito do `admin`.

**S-9 · Unit systemd do `rustployd` quase sem hardening — Média · P**
Só `NoNewPrivileges`, `PrivateTmp` e `CapabilityBoundingSet=CAP_NET_BIND_SERVICE`. Faltam, e cabem
sem mudar comportamento: `ProtectSystem=strict` (+ `ReadWritePaths=/var/lib/rustploy /etc/rustploy
/run/rustploy`), `ProtectHome=yes`, `PrivateDevices=yes`, `ProtectKernelTunables/Modules/Logs`,
`ProtectControlGroups`, `ProtectClock`, `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`,
`RestrictNamespaces`, `RestrictRealtime`, `LockPersonality`, `SystemCallArchitectures=native`,
`SystemCallFilter=@system-service`. **Testar com `systemd-analyze security rustployd`** e uma
sequência de deploy/build real (o `git` e o CLI do Docker/Podman são filhos e herdam as restrições).
`rustployd-fw` (root) já tem `ProtectSystem=strict`; falta `RestrictAddressFamilies=AF_UNIX AF_NETLINK`
(o `ufw` fala com netfilter por `iptables`: validar), `NoNewPrivileges`, `PrivateNetwork` se possível,
`CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW`. O `rustployd-update` já está bem (rede cortada,
pacote assinado com minisign).

**S-10 · Dependências: nada audita — Média · P**
O CI roda `fmt/clippy/test`, **não `cargo audit` nem `cargo deny`** (não estão nem instalados aqui).
`serde_yaml 0.9` está arquivado pelo autor. *Correção:* `cargo-deny` (advisories + licenças + fontes)
no CI dos três repos e no glacier-ui, rodando também por agenda semanal; trocar `serde_yaml` por
`serde_yml`/`yaml-rust2`+serde quando houver um motivo, não por pressa.

**S-11 · Token estático único, sem rotação/revogação por cliente; HTTP em claro permitido — Média · G**
Um token (mais a mestra opcional) para todos os clientes, sem expiração, sem "último uso", sem
revogar um sem trocar todos. A API aceita **bind público sem TLS** (`api.domain` é opcional; o guard só
exige token) — o Bearer atravessa a internet em claro e a GUI aceita `http://` para qualquer host.
*Correção:* (a) recusar (ou avisar bem alto) bind não-loopback sem `api.domain`; a GUI recusar `http://`
para host não-loopback sem confirmação; (b) tokens nomeados no banco, guardados como SHA-256 (o
registry já faz exatamente isso em `registry_tokens`), com nível, `last_used_at` e revogação — o
agente MCP ganha um token próprio, revogável; (c) log de auditoria de comandos mutáveis (quem/quando).

**S-12 · Rootless — Média · G (estratégico)**
O ganho de segurança de maior alcance é rodar o motor **sem root equivalente**: Podman rootless
(o backend Podman já existe, v0.1.3). Sai o "grupo docker = root", o daemon passa a ser contido pelo
usuário. Custo: portas < 1024 (já há `CAP_NET_BIND_SERVICE`), rede entre containers, `ufw`. Vale
um plano próprio.

**S-13 · Segredos em memória/em repouso — Baixa · M**
A identidade `age` fica em claro em `/etc/rustploy/age.key` (inerente; o `recovery_recipient` cobre o
backup). Valores decifrados vivem em `String` comum — usar `secrecy`/`zeroize` nos pontos quentes
(`SecretsManager::decrypt`/`resolve`). O snapshot do SSE manda variáveis **em claro a cada 2 s**
(`/api/events`, só para a mestra): mandar só os nomes e buscar o valor sob demanda reduz superfície e
tráfego (ver P-1).

**Pontos fortes confirmados (não mexer):** comparação de token em tempo constante; recusa de bind
público sem token; updater com assinatura minisign, em processo separado, sem rede; `unsafe` só nos 2
`from_raw_fd(3)` da ativação por socket; `Mutex` envenenado tratado em `queue.rs`; credencial de
provider só vai ao host do próprio provider; zip valida `..`; webhook com token de 192 bits em
tempo constante.

### glacier-ui e rustploy-gui

**G-1 · Tokens de todos os servidores em JSON claro, com permissão padrão — Média · P**
`remember_server` grava `{url, token}` em `saved_servers` via `storage.set`, que faz `std::fs::write`
(modo `0666 & ~umask`, normalmente `0644`) e **não é atômico** (queda no meio = arquivo truncado = perde
todos os logins). *Correção no glacier-ui:* criar o arquivo com `0600` (`OpenOptionsExt::mode`),
escrever em `.tmp` + `rename`. *Correção melhor:* `secret_set/secret_get` no motor sobre o chaveiro do
SO (Secret Service/libsecret no Linux, Keychain, Credential Manager), e o rustploy-gui guardar só o
token ali.

**G-2 · Luau sem sandbox, sem limite de memória e sem interrupção — Média · M**
`Lua::new()` sem `luau.sandbox(true)`, `set_memory_limit` nem `set_interrupt`. As janelas do iced
dividem **uma** thread de UI, então um `while true do end` (bug, não ataque) **congela o app inteiro**,
e uma tabela crescendo sem fim come a RAM. *Correção:* `set_interrupt` com prazo por chamada de handler
(ex.: 2 s → erro de Lua em vez de travar), `set_memory_limit` generoso (256 MiB), e
`sandbox(true)` depois de instalar os globais do motor (globais viram somente-leitura, isola módulos).
Já existe o teste `sandbox_nao_expoe_io_nem_os_execute`; estender para os três.

**G-3 · FS irrestrito no Luau — Baixa (hoje) · M**
`write_file`, `append_file`, `zip_dir`, `zip_write`, `fetch("file://…")`: qualquer caminho. Aceitável
enquanto o script é do app e o diretório de scripts não é gravável por terceiros (o `.deb` instala em
`/usr/share/rustploy`, de root). Vira risco se um dia scripts vierem de fora (plugins, temas) ou se a
GUI rodar de uma pasta de trabalho gravável. *Correção:* `.fs_roots([...])` no builder do host (padrão
liberal, para não quebrar ninguém; o rustploy-gui declara config dir + pastas escolhidas por
`pick_folder`/`save_file`); resolver `..` e symlinks (`canonicalize`) antes de comparar.

**G-4 · `fetch`/SSE sem teto de memória por padrão — Baixa/Média · P**
`max_bytes` é opcional; sem ele `response.into_body().collect()` guarda a resposta inteira. O buffer do
SSE (`buf.drain(..pos+2)`) cresce sem limite se o servidor nunca mandar a linha em branco.
*Correção:* teto padrão (ex.: 64 MiB no `fetch`, 1 MiB por evento SSE), sobrescrevível. **Confirmado:**
não há `danger_accept_invalid_certs` nem verificador desligado — TLS valida com `webpki-roots`.

**G-5 · `require` file-relative — não verificado.** Conferir que `require("../../x")` não escapa da
raiz de módulos em `resolve_module`/`normalize_modname` (só li a parte de busca, não a normalização).

---

## Parte 2 — Desempenho e economia de recursos

### Daemon

**P-1 · O snapshot de 2 s é o maior desperdício do sistema — Alta · M**
`api/http/mod.rs::events` cria, **por conexão SSE**, um laço que a cada 2 s chama `snapshot()`:
~15 `dispatch`, uma `ServiceList` **por projeto** (N+1), três listagens de Docker (imagens, volumes,
redes) + containers, repos do registry, auditoria de infra, status do git de estado, jobs, etc., tudo
serializado em um JSON só — **mesmo que nada tenha mudado**, e multiplicado pelo número de clientes
(GUI + webui + abas). No lado do cliente a GUI decodifica o JSON inteiro em Luau e re-codifica várias
listas a cada tick (`handlers/stream.luau::apply_snapshot`), na thread de UI.
Em ordem de ganho/esforço:
1. **Um snapshot por tick, compartilhado.** Uma task única calcula e publica num `watch<Arc<str>>`;
   cada conexão só repassa. Calcula **somente se há assinantes**.
2. **Só mandar o que mudou.** Hash por seção (`status`, `deployments`, `projects`, `services`,
   `docker_*`, `registry_*`, `jobs`, `infra_audit`, `state_git`…) e enviar apenas as seções cujo hash
   mudou (`{"kind":"snapshot","rev":N,"patch":{…}}`); o cliente mescla. Cai de ~centenas de KiB para
   bytes no caso ocioso, e a GUI deixa de decodificar/re-codificar à toa.
3. **Tirar do tick o que não é "tempo real".** `docker_images/volumes/networks`, `registry_repos`,
   `infra_audit` e `state_git` mudam por ação do usuário ou por evento: empurrar pelo barramento
   (`Event`) quando mudam, ou buscar sob demanda quando a tela abre.
4. **Intervalo adaptativo.** Backoff (2→5→10 s) quando nada muda; o cliente pode pedir
   `?interval=` quando a janela está em segundo plano.
5. **Resolver o N+1** de `ServiceList` por projeto com uma consulta única.
6. O SSE **não é comprimido** (só o `json_response`); depois do item 2 o ganho some, mas numa
   conexão remota o snapshot completo inicial ainda vale gzip.

**P-2 · Ingress abre uma conexão TCP nova por requisição — Alta · M**
`forward` faz `TcpStream::connect` + `http1::handshake` + `spawn(conn)` **a cada request**, sem pool nem
keep-alive para o backend. Custa 1 RTT + 1 task + 1 socket em `TIME_WAIT` por request; sob carga
esgota portas efêmeras. `hyper-util` com `client-legacy` já está nas dependências: usar
`hyper_util::client::legacy::Client` com pool (por backend), ou manter um `SendRequest` por backend.
Junto: **WebSocket/`Upgrade` não funcionam** (nem o servidor nem o cliente usam `with_upgrades()`), o
que é falha funcional para qualquer app com websocket atrás do ingress — confirmar com um teste e
incluir no mesmo trabalho. Opcional: HTTP/2 no TLS (ALPN hoje só `http/1.1`) e compressão.

**P-3 · SQLite — Média · P (medir antes)**
`connect()` usa WAL e `foreign_keys`, mas `synchronous` fica no padrão (`FULL`; em WAL, `NORMAL` é
seguro contra corrupção e evita fsync a cada commit), não define `busy_timeout`, usa o pool padrão
(10 conexões; SQLite escreve uma por vez) e não roda `PRAGMA optimize`. *Fazer:*
`.synchronous(Normal)`, `.busy_timeout(5s)`, `wal_autocheckpoint`, `optimize` ao fechar/periódico.
Os logs já saem do SQLite (segmentos em arquivo com thread dedicada — bom).

**P-4 · Árvore de dependências inchada — Média · P**
- `sqlx` com `runtime-tokio-native-tls`: **puxa `native-tls`/OpenSSL só para falar com SQLite**, que
  não usa TLS. Trocar por `runtime-tokio`. (Confirmado por `cargo tree -i native-tls`.)
- **Dois backends de criptografia**: `rcgen` com `aws_lc_rs` (compila `aws-lc-sys`, C/C++ pesado) e o
  resto em `ring`. Usar a feature `ring` do `rcgen` e tirar o `aws-lc-sys`.
- `tokio = "full"` e `hyper = "full"`: listar só as features usadas.
- `reqwest` ainda está no `Cargo.toml` (a memória do projeto dizia "só hyper, exceção `gitea.rs`"):
  confirmar se `gitea.rs` é o único uso e migrar quando for conveniente — tira mais uma árvore de TLS/HTTP.
- 377 crates únicos no daemon. Efeito esperado: build bem mais rápido, binário menor, menos CVEs a
  acompanhar. **Não medi o tamanho** (não havia binário de release local): rodar
  `cargo bloat --release --crates` antes e depois.
- Perfil de release: `opt-level=3`, `lto=true`, `codegen-units=1` já estão; falta `strip = "symbols"`
  (binário menor, sem custo de execução). `panic = "abort"` **não** (o `mlua` exige `unwind`, está
  comentado no `Cargo.toml`) — só se o daemon ganhar um perfil só dele.

**P-5 · Recursos dos serviços e do host — Média · M**
Sem limite padrão de memória/CPU/PIDs e **sem rotação de log do container** (ver S-8): um serviço
verborrágico enche `/var/lib/docker`. Uma política de default por tipo (`app`: mem alta mas finita;
bancos: sem limite de swap) e a rotação `json-file` resolvem. O aquecimento de builds (v0.1.7) e as
imagens em cache (`image_cache = 2`) ocupam disco: vale um painel de "espaço usado pelo rustploy".

**P-6 · Métricas a cada 2 s — Baixa · P (verificar)**
`[metrics] interval_secs = 2` com `sysinfo`: conferir se a coleta roda sempre ou só com assinante.
Se sempre, pausar sem clientes e subir o intervalo para 5–10 s em ocioso.

### glacier-ui / rustploy-gui

O motor **já é cuidadoso**: `EvalCache` com época e dependências, `FxMap`, `virtualize`, instrumentação
`GLACIER_PERF`, JSON lazy no `ctx`. Os ganhos que sobram:

**P-7 · `sync_to_luau`/`sync_from_luau` são O(chaves do contexto) a cada handler — Média · M**
Cada execução de handler **limpa e repopula a tabela `ctx` inteira** (cópia de string por chave, JSON
poupado pelo `instalar_ctx_preguicoso`) e `sync_from_luau` percorre todas as chaves de volta. Com um
contexto grande (o snapshot grava dezenas de chaves), cada clique paga isso. *Correção:* rastrear
chaves sujas (o `RenderInputs` já tem época) e só reescrever o delta; manter a tabela viva entre
execuções. **Medir primeiro** com `GLACIER_PERF=1` num clique com a tela de serviços aberta.

**P-8 · Um engine Luau por janela, prelúdio recompilado — Baixa/Média · M**
`build_with` faz `Lua::new()` + `load(PRELUDE)` + instala todos os globais **para cada janela/componente
com script** (a janela de logs, por exemplo). Compilar o prelúdio uma vez para bytecode
(`mlua::Compiler`) e reutilizar economiza CPU na abertura e RAM por janela; medir a RAM de um engine.

**P-9 · Decodificação do snapshot — ver P-1.** Com patches por seção, `apply_snapshot` só reprocessa o
que mudou; hoje ele roda inteiro a cada 2 s na thread de UI.

**P-10 · `storage` lê e grava o arquivo inteiro a cada `get/set` — Baixa · P**
Documentado como "não pensado para alta frequência", mas é síncrono na thread de UI. Cachear o mapa em
memória e gravar com debounce + atômico (junto com G-1).

---

## Parte 3 — Ordem sugerida

| # | Item | Ganho | Esforço |
|---|---|---|---|
| 1 | **S-1** token do git fora do log/argv | fecha vazamento real | P |
| 2 | **S-3 + S-4** timeouts, teto de conexões e de corpo | fecha DoS barato | M |
| 3 | **P-1** snapshot compartilhado + por seção | maior economia de CPU/rede/UI | M |
| 4 | **P-2** pool no ingress (+ WebSocket) | latência e estabilidade sob carga | M |
| 5 | **S-9** hardening do systemd | superfície do daemon | P |
| 6 | **S-8 + P-5** hardening e limites de container, rotação de log | segurança + disco | M |
| 7 | **P-4** dependências (sqlx sem TLS, 1 backend cripto, `strip`) | build/binário/CVEs | P |
| 8 | **G-1 + G-2** storage 0600/atômico/chaveiro; Luau com interrupt+memória | robustez da GUI | M |
| 9 | **S-2, S-5, S-6, S-7, S-10** | endurecimento geral | P cada |
| 10 | **P-3** SQLite `NORMAL` + `busy_timeout` | pequeno, quase de graça | P |
| 11 | **S-11** tokens nomeados + TLS obrigatório; **S-12** rootless | estratégico | G |

## O que medir antes de decidir (nada disso foi medido)

1. Tamanho/tempo do `snapshot()` com ~20 serviços: bytes por tick e CPU do daemon com 1 e com 3 clientes.
2. `GLACIER_PERF=1` na GUI ociosa com a tela de serviços aberta (custo do `apply_snapshot` a cada 2 s).
3. `cargo bloat --release --crates` e `cargo tree -d` no daemon.
4. `wrk`/`oha` contra um serviço atrás do ingress (conexões novas/s com e sem pool).
5. `systemd-analyze security rustployd` antes e depois de S-9.
6. `cargo deny check` e `cargo audit` uma vez, à mão, para ver o passivo atual.
