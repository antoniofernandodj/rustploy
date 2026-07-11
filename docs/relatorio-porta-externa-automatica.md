# Relatório: URL de conexão externa sem burocracia — porta automática + firewall gerenciado pelo rustploy

> Documento de design da feature. Escrito antes da implementação para registrar o
> raciocínio completo: o problema, por que ele existe (em nível de sistema
> operacional), as alternativas descartadas e o desenho detalhado da solução.

## 1. O problema

Hoje, para conectar o DBeaver (ou qualquer cliente) da máquina local a um banco
rodando no rustploy, o fluxo é:

1. Criar o serviço de banco (wizard) e fazer o deploy — até aqui, tudo dentro do rustploy. ✅
2. Abrir as configurações do serviço e **digitar manualmente uma "porta externa"**
   (`host_port`) — escolhendo um número na sorte, com risco de colidir com outra
   porta já usada por outro serviço, pelo próprio daemon ou por qualquer processo
   da VPS. ❌
3. **Conectar na VPS via SSH e rodar `sudo ufw allow <porta>`** — exatamente o tipo
   de tarefa de sysadmin que o rustploy existe para eliminar. ❌

Só depois disso a "URL de conexão externa" mostrada na aba do serviço passa a
funcionar. O objetivo desta feature é reduzir o fluxo a: **criar o banco → deploy
→ copiar a URL → colar no DBeaver → funcionar para sempre**.

## 2. Como funciona hoje (e por que o UFW bloqueia)

### 2.1 O rustploy NÃO publica a porta via Docker

Há duas formas de expor a porta de um container para fora da máquina:

- **Publicação Docker (`-p 5432:5432`)**: o Docker cria regras de NAT no kernel
  (chain `PREROUTING`/`FORWARD` do netfilter). É o que Dokploy/Coolify fazem.
- **Proxy no host**: um processo do host escuta na porta e repassa os bytes para o
  container. É o que o **rustploy** faz.

No rustploy, quando um serviço tem `host_port` configurado, o próprio daemon abre
um listener TCP em `0.0.0.0:<porta>` e repassa cada conexão, byte a byte, para o
IP do container live:

```
crates/daemon/src/ingress/proxy.rs   → serve_port_proxy()      (o listener)
crates/daemon/src/ingress/router.rs  → upsert_port_route()     (a tabela porta→backend)
crates/daemon/src/deploy/executor.rs → chama upsert_port_route no swap do deploy
```

Esse design tem duas vantagens importantes:

1. **Swap sem downtime**: no redeploy, o listener continua o mesmo; só a tabela
   interna (um `ArcSwap`) passa a apontar para o container novo. Conexões novas já
   vão para o novo backend, sem soltar/reabrir a porta.
2. **A config do DBeaver nunca quebra**: o IP do container muda a cada deploy, mas
   o `host:porta` que o DBeaver conhece é o do proxy, que é estável.

### 2.2 O efeito colateral: a chain INPUT

O preço desse design: tráfego destinado a um **processo do host** passa pela chain
`INPUT` do netfilter — que é exatamente onde o UFW aplica suas regras (política
padrão: `deny` para conexões de entrada). Já as portas publicadas pelo Docker fazem
NAT **antes** da chain INPUT e a contornam (o famoso comportamento "Docker ignora
UFW" — considerado por muita gente uma falha de segurança do Docker, não uma
virtude).

Ou seja: o rustploy tem a arquitetura *mais correta*, e é justamente por isso que
o UFW o bloqueia. A solução não é abandonar o proxy — é fazer o rustploy cuidar
das duas pontas que hoje são manuais: **escolher a porta** e **liberar o
firewall**.

### 2.3 Por que o daemon não pode simplesmente rodar `ufw allow`

O daemon roda deliberadamente **sem privilégios** (`packaging/rustployd.service`):

```ini
User=rustploy
Group=rustploy
NoNewPrivileges=yes                        # bloqueia sudo/setuid para sempre
AmbientCapabilities=CAP_NET_BIND_SERVICE   # única capability: bind em porta <1024
```

`ufw` exige root. `NoNewPrivileges=yes` impede qualquer escalação (nem `sudo`
funcionaria). Abrir mão desse hardening rodando o daemon inteiro como root seria
um retrocesso — o Docker daemon já é criticado exatamente por isso. A saída
padrão da indústria para esse dilema é um **helper privilegiado de superfície
mínima**: um segundo processo, esse sim root, que só sabe fazer UMA coisa
(liberar/bloquear uma porta TCP) e só aceita ordens do daemon.

### 2.4 Alternativas consideradas e descartadas

| Alternativa | Por que foi descartada |
|---|---|
| Publicar a porta via Docker (`-p`) | Depende do bypass Docker-sobre-UFW (comportamento considerado falha de segurança, quebra em servidores endurecidos com ufw-docker); o binding é fixado na criação do container, então staging e live não podem coexistir na mesma porta → perde o swap sem downtime e piora o rollback. |
| Rodar o daemon como root | Joga fora o hardening deliberado do unit; superfície de ataque gigante (o daemon fala HTTP com a internet). |
| Daemon manipular nftables numa tabela própria | Não funciona: no nftables, `accept` numa tabela não impede o `drop` de outra tabela no mesmo hook — a tabela do UFW ainda derrubaria o pacote. |
| Túnel via API (estilo `kubectl port-forward`) | Resolve o acesso do desenvolvedor sem abrir porta nenhuma, mas exige um processo cliente rodando na máquina local — não entrega uma "URL externa que sempre funciona" para colar direto no DBeaver. Fica como evolução futura possível, complementar. |
| SSH tunnel nativo do DBeaver | Funciona hoje sem mudança nenhuma, mas o usuário definiu que não quer depender de SSH. |

## 3. Visão geral da solução

Duas dores → duas peças novas, mais a cola na UI:

```
┌─────────────────────────────────────────────────────────────────────┐
│  GUI (wizard)          "☑ Gerar URL de conexão externa"             │
│        │                                                            │
│        ▼  WizardCreate { expose_external = true }                   │
│  daemon: build_spec()  → host_port = Some(0)   ← sentinela "auto"   │
│        │                                                            │
│        ▼                                                            │
│  service_create → ports::allocate()  → ex.: 20001  (Parte 1)        │
│        │              (varre a faixa 20000-20999, pula ocupadas)    │
│        ▼                                                            │
│  firewall::ensure_allowed(20001)     (Parte 2)                      │
│        │  JSON via /run/rustploy/fw.sock                            │
│        ▼                                                            │
│  rustployd-fw (root)  →  ufw allow 20001/tcp comment 'rustploy'     │
│                                                                     │
│  deploy → upsert_port_route(20001, ip_do_container:5432)            │
│                                                                     │
│  GUI: URL externa pronta:                                           │
│    jdbc:postgresql://<vps>:20001/meudb?user=app&password=...        │
└─────────────────────────────────────────────────────────────────────┘
```

## 4. Parte 1 — Alocação automática de porta

### 4.1 A sentinela `host_port = 0`

O campo `ServiceSpec.host_port` é `Option<u16>`. Passamos a dar significado ao
valor `Some(0)` (porta 0 não é utilizável na prática — no TCP ela significa "o SO
escolhe"): **"rustploy, aloque uma porta para mim"**.

Por que uma sentinela em vez de um campo novo (`host_port_auto: bool`)? Por causa
do formato de wire: o protocolo TUI↔daemon usa **postcard, que é posicional** —
os tipos de `Command`/`Response` não admitem `serde(default)`/`skip`, então
qualquer campo novo em `ServiceSpec` obriga a atualizar todos os pontos que
constroem o spec (TUI, GUI, importer, manifests, testes). A sentinela evita isso:
o schema não muda, e `0` nunca foi um valor válido antes.

Importante: **a porta alocada é persistida no spec**. Depois da criação,
`host_port` contém a porta real (ex.: 20001) — não fica "0" no banco. É isso que
garante estabilidade: redeploys, restarts do daemon e reboots da VPS reusam a
mesma porta para sempre.

### 4.2 A faixa dedicada (configurável)

Nova seção no config (`crates/shared/src/config.rs` + `packaging/config.toml`):

```toml
[external_ports]
range_start = 20000
range_end   = 20999
```

Uma faixa dedicada tem três méritos:

- **Zero colisão com o mundo**: 20000-20999 não conflita com portas notórias
  (5432, 3306, 6379, 8080, 9000...) nem com as do próprio rustploy (8080/443
  ingress, 8788 webhook, porta da API).
- **Auditável**: o admin sabe que qualquer regra `2xxxx/tcp` no `ufw status` com
  comment `rustploy` é gerenciada — e o helper (Parte 2) **se recusa** a tocar em
  portas fora da faixa, limitando o estrago mesmo se o daemon for comprometido.
- **1000 portas** = 1000 serviços expostos, mais que suficiente para single-node.

### 4.3 O algoritmo (`crates/daemon/src/ports.rs`, novo)

```
allocate(db, faixa) -> Result<u16, String>:
  1. usadas = host_port de TODOS os serviços no SQLite
             + portas reservadas do daemon (http, https, api, webhook)
  2. para cada p em range_start..=range_end:
       se p ∈ usadas           → pula   (reservada por outro serviço)
       se TcpListener::bind(("0.0.0.0", p)) falha → pula (algum processo
                                 do SO já ocupa; o bind de teste é solto
                                 na hora, é só uma sondagem)
       senão → retorna p
  3. faixa esgotada → erro claro
```

Dois detalhes:

- O teste de bind cobre o caso de um processo alheio (fora do rustploy) já estar
  na porta — coisa que só olhar o banco não pegaria.
- Corrida teórica entre a sondagem e o listener real subir no deploy: irrelevante
  na prática (single-node, janela de milissegundos, faixa dedicada), e o pior
  caso é o mesmo erro de bind que já existe hoje com porta manual.

### 4.4 Onde a alocação acontece

- **`api/handlers/service_create.rs`**: se o spec chegar com `host_port == Some(0)`,
  aloca e substitui pela porta real **antes** de persistir. Se chegar com porta
  manual já usada por outro serviço → erro de validação (hoje isso passa batido e
  o segundo listener falha silenciosamente no deploy).
- **`api/handlers/service_update.rs`**: mesma lógica (o usuário pode ligar a
  exposição depois, na aba de settings do serviço).
- **Wizard** (`crates/shared/src/wizard.rs`): `WizardCreateReq` ganha o campo
  `expose_external: bool`; `build_spec` traduz para `host_port: Some(0)`. Como o
  wizard delega a criação a `service_create::handle` (ver
  `crates/daemon/src/api/handlers/wizard.rs`), a alocação é herdada de graça.

## 5. Parte 2 — Firewall gerenciado (o helper privilegiado)

### 5.1 O que é, em uma frase

Um segundo binário, `rustployd-fw`, instalado junto com o daemon, que roda como
**root**, escuta num socket Unix local e só sabe executar duas ordens: *"libere a
porta N no firewall"* e *"remova a liberação da porta N"*.

### 5.2 Por que socket activation do systemd

"Socket activation" = o **systemd** cria e é dono do socket
(`/run/rustploy/fw.sock`) e só inicia o processo helper quando alguém conecta.
Ganhos concretos:

- **Permissões resolvidas pelo systemd**: o socket nasce com dono `root`, grupo
  `rustploy`, modo `0660`. Só root e o daemon (que roda no grupo `rustploy`)
  conseguem conectar. Nenhum outro usuário da máquina consegue mandar ordens.
- **Sem processo root ocioso**: o helper só existe enquanto atende.
- **Zero código de setup**: o helper recebe o socket pronto via descritor herdado.

Dois arquivos novos em `packaging/`:

```ini
# rustployd-fw.socket
[Socket]
ListenStream=/run/rustploy/fw.sock
SocketUser=root
SocketGroup=rustploy
SocketMode=0660

# rustployd-fw.service  (ativado pelo .socket)
[Service]
ExecStart=/usr/libexec/rustployd-fw
# root, com hardening: ProtectHome, PrivateTmp, etc.
```

### 5.3 O protocolo (deliberadamente trivial)

Uma linha JSON por requisição, uma linha JSON de resposta:

```
→ {"op":"allow","port":20001}
← {"ok":true,"backend":"ufw"}

→ {"op":"deny","port":20001}
← {"ok":true,"backend":"ufw"}

→ {"op":"allow","port":443}
← {"ok":false,"error":"porta 443 fora da faixa gerenciada (20000-20999)"}
```

### 5.4 Superfície mínima = segurança

O helper é a única peça root do sistema, então ele é paranóico por construção:

1. **Só allow/deny de TCP.** Não executa comando arbitrário, não recebe strings
   de shell, não tem outros verbos.
2. **Só portas dentro da faixa** `[external_ports]` (ele lê o mesmo
   `/etc/rustploy/config.toml`). Mesmo um daemon comprometido não consegue usar o
   helper para abrir a porta 22 ou fechar a 443.
3. **Regras marcadas**: `ufw allow <p>/tcp comment 'rustploy'` — o admin vê no
   `ufw status` de quem é a regra.

### 5.5 Detecção do firewall

```
ufw instalado e "Status: active"  → usa ufw (allow/delete allow)
ufw inativo ou ausente            → no-op com ok:true, backend:"none"
                                     (sem firewall = porta já alcançável;
                                      não há o que liberar)
```

Suporte a firewalld fica como `TODO` comentado no ponto de detecção (a estrutura
de backends já nasce pronta para ganhar um segundo braço).

### 5.6 O cliente dentro do daemon (`crates/daemon/src/firewall.rs`, novo)

`ensure_allowed(port)` / `ensure_denied(port)`: conecta no socket
(`tokio::net::UnixStream`), manda a linha JSON, espera resposta com timeout curto.

Política de erro: **falha de firewall nunca derruba um deploy nem uma criação de
serviço**. Se o socket não existe (ambiente dev, instalação antiga) ou o comando
falha, o daemon loga um warning e segue — o pior caso é o comportamento atual
(porta bloqueada), nunca pior que hoje. O resultado aparece no deploy log
("porta 20001 liberada no firewall (ufw)" / "não foi possível liberar…").

### 5.7 Ciclo de vida das regras (quando allow, quando deny)

| Evento | Ação de firewall |
|---|---|
| Serviço criado/atualizado com `host_port` | `allow` imediato (a regra pode existir antes do listener; inofensivo) |
| `host_port` alterado no update | `deny` na porta antiga (se nenhum outro serviço a usa) + `allow` na nova |
| `host_port` removido no update | `deny` |
| Serviço deletado | `deny` + `remove_port_route` — **corrige um gap atual**: `service_delete.rs` hoje só remove rotas de domínio, deixando o listener TCP da porta vivo para sempre |
| Boot do daemon (recovery restaura rotas de porta) | `ensure_allowed` para cada porta restaurada — **auto-cura**: se o admin resetar o ufw ou reinstalar o SO, as regras voltam sozinhas no próximo start |
| Parar serviço (sem deletar) | Nada — a porta continua reservada/liberada; é isso que mantém a config do DBeaver estável |

A regra de deny verifica antes se outro serviço compartilha a porta (não deve
acontecer com alocação automática, mas portas manuais continuam permitidas).

## 6. Parte 3 — GUI

1. **Wizard de novo banco** (`views/new_service.xml`, formulário de database +
   `views/scripts/handlers/wizard.luau::ns_create`): checkbox **"Gerar URL de
   conexão externa"** (desligado por padrão — exposição pública é opt-in
   consciente). Marcado → `expose_external = true` no request `WizardCreate`.
2. **Detalhe do serviço → aba de settings** (`views/service.xml`, campo de porta
   externa + `services.luau::dom_hostport_save`): botão **"Automática"** ao lado
   do campo — envia a sentinela 0; a resposta volta com a porta real alocada e os
   campos `svc_host_port`/`svc_external_url` re-renderizam na hora.
3. **URL externa**: já existe e já é completa
   (`views/scripts/fmt/service_detail.luau::external_url` monta
   `jdbc:postgresql://host:porta/db?user=…&password=…` usando o host do `api_url`
   da sessão + `host_port` + credenciais das env vars). **Nenhuma mudança** — ela
   só passa a nascer válida sem etapas manuais.

## 7. O fluxo final, na prática

1. GUI → Novo serviço → PostgreSQL → nome `meudb` → ☑ Gerar URL de conexão
   externa → Criar.
2. O daemon monta o spec (`host_port = 0`), aloca `20001`, persiste, pede
   `allow 20001` ao helper (que roda `ufw allow 20001/tcp comment 'rustploy'`).
3. Deploy: container sobe, `upsert_port_route(20001, "172.x.x.x:5432")` ativa o
   passthrough.
4. Aba do serviço mostra:
   `jdbc:postgresql://minha-vps.com:20001/meudb?user=app&password=s3cr3t`
   → botão Copiar → colar no DBeaver → conecta.
5. **Para sempre**: redeploy troca o backend por baixo do proxy (mesma porta);
   restart do daemon restaura rota e regra de firewall (recovery); reset do ufw é
   auto-curado no próximo boot. A conexão do DBeaver não é reconfigurada nunca.

## 8. Segurança — o que muda e o que fica exposto

- **O que fica exposto**: a porta alocada fica acessível da internet inteira,
  protegida pela autenticação do próprio banco (senha gerada forte pelos
  blueprints). É o mesmo modelo do Dokploy/Coolify com portas publicadas — e é o
  preço inevitável de "URL externa que funciona de qualquer lugar sem cliente
  local". Trade-off explicitamente aceito, e opt-in por serviço.
- **Mitigações**: faixa dedicada + helper que recusa portas fora dela + regras
  comentadas e auditáveis + `deny` automático na remoção (sem regra órfã).
- **Ganho colateral**: hoje o listener de `host_port` já sobe em `0.0.0.0` e a
  única barreira é o UFW manual; com a gestão automática, deixar de expor um
  serviço deletado passa a ser garantido em vez de depender de lembrar do
  `ufw delete`.
- **Limitação honesta**: se a VPS estiver atrás de um firewall **externo à
  máquina** (security group da AWS/Oracle/hetzner cloud firewall), nenhuma regra
  local resolve — isso fica documentado na mensagem de erro/log quando a conexão
  externa não funcionar apesar do `allow` local ter sucedido.

## 9. Arquivos criados/alterados

| Arquivo | Mudança |
|---|---|
| `crates/shared/src/config.rs` | + seção `ExternalPortsConfig` (default 20000-20999) |
| `crates/shared/src/wizard.rs` | + `expose_external` em `WizardCreateReq`; `build_spec` → `host_port: Some(0)` |
| `crates/daemon/src/ports.rs` **(novo)** | alocador de portas |
| `crates/daemon/src/firewall.rs` **(novo)** | cliente UDS do helper (`ensure_allowed`/`ensure_denied`) |
| `crates/daemon/src/api/handlers/service_create.rs` | aloca sentinela 0, valida duplicata, `ensure_allowed` |
| `crates/daemon/src/api/handlers/service_update.rs` | idem + `deny` da porta antiga |
| `crates/daemon/src/api/handlers/service_delete.rs` | + `remove_port_route` (gap atual) + `ensure_denied` |
| `crates/daemon/src/api/handlers/manifest_apply.rs` | sentinela 0 em manifesto: reusa porta do serviço homônimo ou aloca |
| `crates/daemon/src/deploy/recovery.rs` | `ensure_allowed` nas rotas restauradas no boot |
| `crates/daemon/src/deploy/executor.rs` | linha de deploy log sobre o firewall |
| `crates/fw-helper/` **(novo crate)** | binário `rustployd-fw` |
| `packaging/rustployd-fw.socket` / `.service` **(novos)** | socket activation root |
| `packaging/config.toml` | seção `[external_ports]` documentada |
| `packaging/debian/postinst` | enable do `rustployd-fw.socket` |
| `crates/daemon/Cargo.toml` | assets do deb (helper + units) |
| `Makefile` | `deb-daemon` compila também o fw-helper |
| `crates/rustploy-gui/views/new_service.xml` + `scripts/handlers/wizard.luau` | toggle de exposição |
| `crates/rustploy-gui/views/service.xml` + `scripts/handlers/services.luau` | botão "Automática" |
| `crates/client/` (TUI) | atualizar construção de `WizardCreateReq` (campo novo, posicional) |

## 10. Verificação

1. `cargo check --workspace` + `cargo test -p daemon -p shared` (testes novos:
   alocador com colisão/esgotamento; parse da config).
2. Helper isolado: rodar `rustployd-fw` com socket em path de dev e conversar em
   JSON via `socat`; conferir allow/deny reais e a recusa fora da faixa.
3. E2E em VM/VPS com ufw ativo: instalar o .deb, criar postgres pelo wizard com o
   toggle, conferir porta no range + regra no `ufw status` + conexão real
   DBeaver/psql pela URL exibida; redeploy sem tocar no DBeaver; deletar o
   serviço e conferir que a regra e o listener somem.
4. GUI: `luau-lsp analyze` + `cargo test -p rustploy-gui --test templates_render`.
