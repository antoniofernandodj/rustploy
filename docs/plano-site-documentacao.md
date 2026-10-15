# Plano: site de documentação do Rustploy (SSG, SEO e GEO)

> **Status: em implementação** (2026-10-14). O site vive no submódulo
> `rustploy-docs/` (Astro + Starlight, pt-BR). Feitas as fases 0 e 1, a trilha
> de MCP (fase 2) e parte dos guias da fase 3 (Git, zip, Compose, variáveis e
> secrets, healthcheck, domínios/TLS, 502, webhooks, fila, pré-deploy, jobs,
> bancos/brokers, templates, banco compartilhado e migração, IaC, registry,
> limpeza do Docker, backup) e a referência gerada do código (386 templates,
> comandos, respostas, eventos, paridade). Também: resolução de variáveis, `config.toml` conferido contra o código e migração do
> Dokploy. Também: monitoramento e watchdog, segurança do servidor. Seção "Interface desktop" com 10 páginas e 32 screenshots (host e segredos borrados): login,
> projetos, assistente de serviço, tela do serviço, domínios/healthcheck/avançado, Docker e
> registry, Settings, Monitoring e Schedules. Faltam: Deploy Engine, Ingress e a janela de logs
> na GUI (sem screenshot), tour da web UI, referência do manifesto gerada e inglês. Este arquivo não é a documentação: é o
> plano de como ela será feita.
>
> **Achados ao documentar** (viram trabalho de produto): o botão Rollback não
> redeploya; os campos watch paths, build path, build stage e submódulos Git
> estão na tela mas o deploy os ignora; o clone exige `git` no servidor, que o
> `.deb` não declara como dependência; não há GitHub Release, então o
> `install.sh` não funciona; o `AGENTS.md` ainda cita nomes de container antigos; no app desktop o botão Rebuild
> chama a mesma função que Deploy e o UPTIME/SERVICES do painel lateral do serviço são do
> daemon, não do serviço; `config.toml` com erro é
> ignorado em silêncio (sem log) e o daemon sobe com os padrões; o `.env` do
> Compose guarda secrets decifrados em disco durante o deploy; não há variáveis
> de build (`docker build` sem build args); `[metrics].history_points` não tem efeito e as métricas não têm histórico; os
> containers têm restart policy `no` e, pelo código, os serviços não voltam
> sozinhos depois de reiniciar a máquina (autostart não implementado); o importer
> do Dokploy apaga serviços
> de mesmo nome em qualquer projeto e zera as variáveis de projeto homônimo, não
> converte aplicações de imagem Docker e não vai no `.deb`;
> o horário de jobs e da limpeza automática é UTC e a tela não avisa; a opção de
> expor banco externamente só existe na GUI, não na webui; o cache de build não
> tem botão de limpeza manual em nenhum cliente.

## 1. Por que fazer, e para quem

Hoje o conhecimento do Rustploy está espalhado em quatro lugares, e nenhum deles
foi escrito para quem **chega agora**:

| Onde | O que é | Problema para o usuário novo |
|---|---|---|
| `README.md` (daemon) | pitch + build | desatualizado (ainda diz "não tem Web UI"), fala de `cargo build`, não de uso |
| `docs/*.md` (~50 arquivos) | planos, relatórios, lições aprendidas | escritos para quem mexe no código; metade é histórico |
| `AGENTS.md` | guia de desenvolvimento + arquitetura | excelente, mas é para contribuidor |
| a própria GUI/webui | as telas | mostra *o quê*, não *por quê* nem *em que ordem* |

O público do site é outro: **quem vai instalar e operar** um Rustploy numa VPS.
Pressuposto combinado: **sabe Docker** (imagem, container, rede, volume,
Dockerfile, compose). Não sabe o que é um PaaS, não sabe o que o Rustploy faz
por cima do Docker, e quer chegar do zero a "meu app no ar com HTTPS" sem
adivinhar.

Dois públicos secundários, que pesam no desenho:

- **Buscadores** (SEO): quem pesquisa "alternativa leve ao Dokploy", "PaaS
  self-hosted em Rust", "deploy zero-downtime docker sem swarm".
- **IAs que respondem perguntas** (GEO): ChatGPT, Claude, Perplexity, Gemini
  citando o Rustploy. Elas premiam texto factual, autocontido, com
  afirmações verificáveis e estrutura limpa — e o Rustploy tem um trunfo raro:
  o próprio produto é operável por IA (MCP), então a documentação deve ser
  também **legível por máquina**.

## 2. Escolha do framework

**Recomendação: Astro + Starlight.**

| Critério | Starlight | Docusaurus | VitePress | mdBook |
|---|---|---|---|---|
| HTML estático puro, sem hidratação desnecessária | ✅ (zero JS por padrão) | ⚠️ React hidrata tudo | ✅ | ✅ |
| Busca offline sem serviço externo | ✅ Pagefind | ⚠️ Algolia ou plugin | ✅ minisearch | ✅ |
| i18n (pt-BR agora, en depois) | ✅ nativo | ✅ | ✅ | ⚠️ gambiarra |
| Sitemap, canonical, OpenGraph, JSON-LD | ✅ (+ `<head>` customizável) | ✅ | ⚠️ manual | ❌ |
| Gerar páginas a partir de dados (386 templates, `Command`) | ✅ (rotas dinâmicas Astro) | ⚠️ | ⚠️ | ❌ |
| `llms.txt` / `llms-full.txt` | ✅ plugin existente | ⚠️ | ⚠️ | ❌ |
| Dark mode, mobile, acessibilidade de graça | ✅ | ✅ | ✅ | ⚠️ |

mdBook seria a escolha "da casa" (é Rust), mas perde justamente nos pontos que
motivaram o pedido: SEO estruturado, i18n, páginas geradas por dados e GEO.
Docusaurus funciona, mas entrega mais JavaScript do que uma doc precisa.

Hospedagem: qualquer host estático (Cloudflare Pages, GitHub Pages, Netlify).
Recomendo Cloudflare Pages ou GitHub Pages — gratuitos, CDN, HTTPS, deploy por
push. Quem decide é o domínio (ver perguntas).

**Onde o código do site mora:** pasta `site/` no **agregador**, não um quarto
submodule. Motivo: o site *lê* os três repos (índice de comandos, `config.rs`,
catálogo de templates). No agregador ele enxerga tudo sem ponteiro extra, e o
CI do site pode rodar no mesmo `release.yml`/checkout recursivo.

## 3. Princípios editoriais

1. **Diátaxis**: cada página é de *um* tipo só — **tutorial** (aprender fazendo),
   **guia prático** (resolver uma tarefa), **explicação** (entender o porquê) ou
   **referência** (consultar). Misturar é o erro mais comum de doc técnica.
2. **Docker como âncora.** Todo conceito novo entra com a pergunta "o que você
   faria na mão?". Ex.: *deploy* = o que você faria com `docker pull` +
   `docker stop` + `docker run` + trocar o proxy, só que sem janela de
   indisponibilidade. Tabela "Docker → Rustploy" logo no início.
3. **Cada página é autocontida** (GEO): a primeira frase responde a pergunta do
   título, sem depender de "como vimos antes". Uma IA que ler só aquela página
   precisa acertar.
4. **Todo exemplo foi executado.** Comando, YAML e JSON de exemplo são testados
   contra um daemon de verdade no CI (ver §8). Nada de exemplo "provavelmente funciona".
5. **Honestidade sobre limites.** Há uma seção fixa "Limitações conhecidas" em
   cada área (ex.: log de container não persiste, `ServiceUpdate` é
   substituição total, token sem escopo). Isso vira confiança e evita suporte.
6. **Dois clientes, uma explicação.** Cada tarefa é mostrada em abas
   **GUI · Web UI · API/MCP**, porque a paridade não é total (ver §7).
7. **Versionado.** Cada página declara `verificado_em: 0.x.y`. Página sem
   verificação na versão atual é sinalizada no CI.

## 4. Mapa do conteúdo

Numeração = ordem de leitura sugerida (a barra lateral segue esta ordem).
Marcado ⚙️ = gerado a partir do código; 🖼️ = precisa de screenshots.

### 0. Comece aqui
- **O que é o Rustploy** — PaaS single-node, um binário, < 50 MB de RAM.
- **Rustploy vs Dokploy vs Coolify vs CapRover** — tabela honesta (inclui
  "quando *não* usar": multi-host, Kubernetes, builds sem Dockerfile).
- **Mapa mental: Docker → Rustploy** — a tabela âncora (projeto = rede bridge,
  serviço = container(s) + rota, deploy = swap sem downtime…).
- **Glossário** ⚙️ — Projeto, Serviço, Fonte, Deployment, Ingress, Staging,
  Drain, Token de conexão, Job, Template, Servidor compartilhado.

### 1. Instalação e primeiro acesso
- **Requisitos** (Linux, Docker, systemd, portas 80/443/9797, DNS).
- **Instalar na VPS** — `install.sh`, o que ele cria (usuário `rustploy`,
  `/etc/rustploy`, `/var/lib/rustploy`, unit systemd), instalação manual do
  tarball, x86_64 e aarch64. *(verificar antes: o `install.sh` aponta para o
  repo do agregador; confirmar que a URL de release ainda resolve.)*
- **O `config.toml` seção por seção** ⚙️ — `[daemon] [api] [ingress]
  [ingress.acme] [docker] [deploy] [metrics] [secrets] [external_ports]
  [registry] [env_backup]`; precedência (`RUSTPLOY_CONFIG` → `/etc` → `~/.config`);
  overrides por env; **parse tudo-ou-nada** (a pegadinha nº 1); guarda de
  segurança (bind público exige token).
- **Firewall e o `rustployd-fw`** — por que existe um helper root, a faixa
  `20000–20999`, o que ele recusa.
- **Instalar o cliente**: GUI Linux (.deb), GUI Windows (.zip portável, aviso
  de SmartScreen/assinatura), Web UI (já vem no daemon, zero instalação) 🖼️.
- **Primeiro login**: o token de conexão, `rustployd access-key`, onde copiar na
  tela, múltiplos servidores salvos.
- **Atualizar e desinstalar** o daemon e os clientes.

### 2. Tutorial: seu primeiro deploy (15 min)
Fio condutor único, do zero ao HTTPS, com um app de exemplo publicado em
registry público, depois o mesmo com um repo Git. Termina com "o que
aconteceu por baixo" apontando para a trilha 4.
Projeto → serviço (imagem) → porta → healthcheck → domínio → Deploy → logs →
HTTPS → segundo deploy (ver o swap) → rollback.

### 3. Conceitos
- **Projeto**: uma rede bridge dedicada; isolamento entre projetos; env vars
  de projeto vs de serviço; IDs (`prj_`, `svc_`, `dep_`).
- **Serviço e suas 4 fontes**: Registry, Git, Archive (zip), Compose — quando
  usar cada uma (árvore de decisão).
- **Nomes de container** (`rp_<nome>_live`, staging) e **DNS interno**
  (`rp_<safe_name>`) — a base de "como meu app acha o banco".
- **O ciclo de vida de um serviço** (`Stopped → Deploying → Running → Degraded/Error`).

### 4. Guia: criar e configurar serviços
Uma página por assunto, sempre com "Limitações conhecidas".
- **Aplicação a partir de imagem de registry** (Docker Hub, GHCR, privado,
  credenciais).
- **Aplicação a partir de Git**: URL pública; privado com PAT; **GitHub/Gitea
  por OAuth**; branch; `root_path` e `watch_paths` (monorepo); submodules;
  `dockerfile_path`, `build_context`, `build_stage` (multi-stage).
- **Aplicação a partir de zip** (Dockerfile na raiz, teto de 100 MiB).
- **Stack Compose**: o que o Rustploy injeta (rede do projeto), e **qual
  container recebe o domínio** (`ingress_service` / `compose_ingress`) — o caso
  do 502 em stack com vários containers.
- **Templates de um clique**: ~386 apps do catálogo ⚙️ (uma página por
  template — ver §6), variáveis geradas, domínios, volumes.
- **Bancos e brokers pelo wizard**: Postgres, MySQL, MariaDB, MongoDB (réplica),
  Redis; Kafka, RabbitMQ, NATS; usuário/senha gerados; **expor porta externa**
  (faixa automática + firewall).
- **Variáveis de ambiente**: editor `.env`, comentários, projeto vs serviço,
  interpolação `${VAR}`.
- **Secrets**: criação, `secret:NOME`, cifra `age`, chave mestra, o que é e o
  que **não** é protegido, backup da `master.key`.
- **Volumes** (só bind mount — decisão de design) e permissões.
- **Healthcheck**: HTTP, TCP, DockerNative, None; intervalo, timeout, retries,
  start period; como escolher.
- **Réplicas e limites** (CPU shares, memória, `1g`/`256m`).
- **Comando e argumentos** de execução (`run_command`/`run_args`).

### 5. Deploy por dentro (explicação)
- **A máquina de estados** — diagrama (`Pending → … → Live`), o que cada
  estado faz, quanto costuma durar.
- **Zero-downtime na prática**: staging → healthcheck → swap atômico no
  ingress → drain (`drain_secs`) → promote.
- **Rollback**: automático (falha de healthcheck) vs manual (`DeployRollback`).
- **A fila global**: um deploy por vez; promover, reordenar, pausar, abortar;
  jobs na mesma fila.
- **Pré-deploy gate**: rodar um job antes (migração de banco) e só seguir se
  sair 0; múltiplos checks em ordem.
- **Dependências e autostart no boot** — *plano ainda não implementado
  (`docs/plano-dependencias-e-autostart.md`); a página só entra quando for.*
- **Recovery após reinício do daemon**: o que acontece com deploy interrompido.
- **Onde está a causa de uma falha**: build log, histórico de estados, toast,
  status do serviço — e como ler cada um.
- **Logs**: runtime (ao vivo, **não persistido**) vs build (guardado).

### 6. Rede, domínios e TLS
- **Como o ingress funciona**: nenhum container publica porta; o proxy interno
  roteia (por que isso permite zero-downtime).
- **Domínios**: múltiplos por serviço, porta por domínio, TLS por rota; DNS.
- **TLS automático (ACME/Let's Encrypt)**: pré-requisitos (porta 80/443, e-mail),
  emissão, renovação, onde ficam os certificados, TLS na própria porta da API.
- **Portas TCP externas** para bancos/brokers: alocação automática + firewall.
- **Rede entre serviços**: Application ↔ Compose ↔ Compose, exemplos reais
  (URL interna, `ServiceConnectionInfo`).
- **Diagnosticar um 502** — fluxograma: tabela de rotas viva
  (`ingress_routes`) → reconcile (`ingress_reconcile`) → container alvo →
  porta → healthcheck. Esta página é a de maior valor de suporte.

### 7. Dados
- **Servidor de banco compartilhado**: um Postgres para vários projetos,
  databases e usuários dedicados, limite de conexões/timeout.
- **Autorizar projetos** a alcançar o servidor (rede + alias).
- **Migrar um banco antigo** (de um Compose do projeto) para um database
  gerenciado: passos, observação, **rollback**, descarte.
- **Backup automático de env vars** e restauração.
- **O que o Rustploy *não* faz**: backup dos dados dos bancos (dizer isso
  explicitamente e apontar a prática recomendada).

### 8. Automação e CI/CD
- **Webhooks de deploy**: URL por serviço, regenerar token, domínio da API;
  receitas GitHub, GitLab, Gitea, Docker Hub; filtrar por branch; GitHub
  Actions completo.
- **Jobs / Schedules**: one-shot via compose; manual, intervalo, diário,
  semanal; env e rede do projeto; fonte Git; **aquecer (warm)**; cancelar;
  histórico e logs; serviço gatilho.
- **Infra as Code**: manifesto `rustploy.yml` + `.env`; exportar tudo,
  importar, `prune`, `deploy`, segredos redigidos; fluxo GitOps.
- **Copiar um serviço entre servidores** (bundle): o que atravessa, o que não.

### 9. Registry Docker embutido
Habilitar, domínio/TLS, tokens `pull`/`push` (segredo exibido uma vez),
`docker login`/`push`, usar a imagem num serviço, tags, GC, limites. *(Integração
com o executor de deploy e push-to-deploy: confirmar status antes de escrever.)*

### 10. Operação do dia a dia
- **Monitoramento**: CPU/RAM do host e por container, histórico.
- **Inventário Docker** do host e **limpeza** (manual e automática agendada).
- **Fila e saúde do daemon** (`DaemonStatus`, Deploy Engine).
- **Segurança**: modelo de ameaça, token único sem escopo, secrets, isolamento
  por rede, o que fazer se o token vazar, hardening recomendado (firewall, TLS
  na API, VPN/reverse proxy na frente).
- **Backup e restauração do próprio Rustploy**: o que copiar
  (`/var/lib/rustploy/db`, `master.key`, `config.toml`).
- **Solução de problemas** (FAQ por sintoma).

### 11. As interfaces
- **Tour da GUI desktop** 🖼️: login, sidebar, projeto, serviço (abas General,
  Deployments, Environment, Domains, Healthcheck, Logs, Connection, Databases,
  Advanced), janelas auxiliares, bandeja do sistema, notificações de deploy.
- **Tour da Web UI** 🖼️ (PWA, instalável, janelas flutuantes).
- **Qual usar?** e **tabela de paridade** ⚙️ (gerada de `comandos.md`):
  o que só existe em um dos dois e o que só existe por API.

### 12. Agentes de IA e MCP  ⭐ (trilha de destaque)
É a funcionalidade mais nova e o maior diferencial; merece trilha própria,
não uma página perdida.
- **O que é e por que importa**: operar o servidor conversando — listar,
  disparar deploy e *esperar o resultado*, ler build log, diagnosticar 502.
- **Conectar o Claude Code** (passo a passo): onde pegar o token de conexão,
  o comando pronto, `claude mcp list`, abrir sessão nova, `/mcp`.
- **Conectar sem GUI** (só o token): o prompt de validação.
- **Várias instâncias** (um MCP por servidor, nomes, confirmar o alvo antes de
  agir).
- **Referência das ferramentas** ⚙️: `status`, `list_services`, `list_deploys`,
  `deploy`, `build_logs`, `ingress_routes`, `ingress_reconcile`, `rpc`,
  `manual` — argumentos, resposta, exemplos reais.
- **Receitas** (prompts que funcionam): "faz deploy do `api` e me diz por que
  falhou", "o domínio dá 502", "o que mudou nos últimos deploys", "crie um
  serviço a partir deste template", "exporte o manifesto".
- **Sem cliente MCP**: JSON-RPC por `curl`, `/api/login`, `/api/rpc`.
- **Segurança de agentes** (a página que mais importa): o token é de
  administrador, sem escopo nem confirmação em comandos destrutivos; rede
  confiável; revogação; o que está no roadmap (token com escopo).
- **Escrever bons prompts para operar infra**.
- **Outros clientes MCP** (Cursor, etc.) — somente após testar de fato.

### 13. Migrar de outra plataforma
Importer do **Dokploy**: o que converte, o que vira aviso (bloqueante/atenção),
passo a passo, validação pós-importação.

### 14. Referência ⚙️ (quase toda gerada do código)
- **`config.toml` completo** (de `config.rs`: chave, tipo, default, exemplo).
- **API HTTP**: rotas, autenticação, SSE (`snapshot` vs `bus`), gzip, upload.
- **Protocolo**: cada `Command`/`Response`/`Event` (de `comandos.md` +
  `protocol.rs`) — uma entrada por comando com exemplo JSON.
- **Modelos**: `ServiceSpec` e filhos, `Job`, `Healthcheck`…
- **Esquema do manifesto YAML** (de `manifest.rs`).
- **Catálogo de templates**: índice + uma página por template ⚙️ (§6).
- **Códigos de erro** e mensagens comuns com a causa e a correção.
- **Variáveis de ambiente do daemon** e **CLI** (`rustployd`, `access-key`).

### 15. Para quem vai contribuir
Resumo público da arquitetura (da Parte 3 do `AGENTS.md`), como compilar, como
rodar os testes, convenções. O `AGENTS.md` continua sendo a fonte; o site só
aponta/condensa.

## 5. Estrutura fixa de cada página

```
título = pergunta ou tarefa ("Como fazer deploy a partir de um repo Git privado")
resumo de 1–2 frases que já responde (vira meta description e trecho citável)
pré-requisitos (links)
passos numerados / explicação
exemplo completo copiável (testado)
[abas] GUI · Web UI · API/MCP
Limitações conhecidas
Se deu errado (3 sintomas mais prováveis)
Próximos passos (links)
metadados: verificado_em, última atualização, tempo de leitura
```

## 6. Geração a partir do código (o que não se escreve à mão)

Escrever à mão o que o código já sabe garante divergência — o `AGENTS.md`
mesmo documenta que isso já aconteceu com `CLAUDE.md`. Então:

| Página | Fonte | Como |
|---|---|---|
| ~386 páginas de template | `rustploy-shared/templates/blueprints/` | script gera `.mdx` por template: descrição, porta, variáveis, volumes, domínios, compose |
| Referência de `Command` | `docs/indice/comandos.md` + `protocol.rs` | script do `tools/indexer` emite JSON consumido pelo Astro |
| `config.toml` | `rustploy-shared/src/config.rs` | idem, com defaults |
| Esquema do manifesto | `manifest.rs` | idem |
| Ferramentas MCP | `api/agent/tools.rs` / `tools/list` de um daemon real | extraídas do próprio `tools/list` no CI |
| Tabela de paridade | `comandos.md` | idem |
| Manual do agente | `api/agent/manual.md` | incluído como está em uma página `/agentes/manual` |

**Bônus de SEO:** cada template vira uma página própria ("Como hospedar Plausible
/ n8n / Uptime Kuma com Rustploy"). São centenas de páginas com intenção de busca
real e conteúdo único (variáveis, compose, volumes). É o maior ganho orgânico
barato do plano.

## 7. SEO

- **Estático puro** (SSG), HTML semântico, um `<h1>` por página, hierarquia
  de títulos limpa.
- **URLs estáveis em português e sem data**: `/guia/git-privado/`, nunca
  `/p?id=12`. Redirecionamentos mantidos quando renomear.
- `<title>` e `meta description` únicos (vindos do resumo da página);
  `canonical`; **Open Graph/Twitter cards** com imagem gerada por página.
- **`sitemap.xml`** + `robots.txt`; páginas geradas entram no sitemap.
- **JSON-LD**: `SoftwareApplication` na home, `TechArticle` nos guias,
  `HowTo` nos tutoriais passo a passo, `FAQPage` na FAQ, `BreadcrumbList`.
- **Desempenho (Core Web Vitals)**: sem JS desnecessário, imagens em
  AVIF/WebP com dimensões declaradas, fontes self-hosted.
- **Busca interna** Pagefind (roda no navegador, sem serviço de terceiros).
- **Links internos densos** (cada conceito linka a página canônica dele).
- **Páginas de comparação** ("Rustploy vs Dokploy", "vs Coolify", "vs
  CapRover") e **"alternativa a …"** — intenção de busca alta; só com fatos
  verificáveis, sem exagero.
- **i18n**: pt-BR primeiro (idioma do autor e do público de partida); inglês
  como fase posterior, com `hreflang`. Inglês dobra o alcance e é onde a
  maior parte da busca por "self-hosted PaaS" acontece.

## 8. GEO — otimização para motores generativos

O que faz uma IA citar corretamente uma documentação:

1. **`/llms.txt`** (índice curado para LLMs) e **`/llms-full.txt`** (a doc
   inteira em Markdown), gerados no build. Mais, cada página disponível em
   `.md` puro (`/guia/git-privado.md`).
2. **Respostas diretas no topo** de cada página (o "resumo" do §5): é o trecho
   que a IA copia.
3. **Fatos explícitos e datados**: "Desde a versão X…", "padrão: 9797",
   "faixa 20000–20999". Ambiguidade vira alucinação.
4. **Vocabulário consistente** (glossário único; um nome por conceito), com
   definição na primeira ocorrência.
5. **Blocos de código completos e copiáveis**, com a saída esperada logo abaixo.
6. **Tabelas** para comparações e referências (modelos extraem melhor que de prosa).
7. **FAQ por sintoma** em formato pergunta/resposta (casa com prompts de usuário).
8. **Página "Para agentes"**: o manual MCP, o protocolo e uma nota
   "se você é uma IA lendo isto" com as regras de segurança (confirmar antes de
   destrutivos, confirmar o servidor-alvo). Coerente com o produto.
9. **Autoria e proveniência**: repositório público, changelog, data e versão
   em cada página (sinal de confiabilidade para rankers e para IAs).
10. **Permitir rastreadores de IA** no `robots.txt` (GPTBot, ClaudeBot,
    PerplexityBot…) — decisão de política, ver perguntas.
11. **Medir**: perguntar periodicamente a 4–5 IAs um conjunto fixo de 20
    perguntas sobre o Rustploy e registrar acerto/erro (planilha no repo).
    Onde errarem, a página correspondente está ambígua — corrige-se a página.

## 9. Qualidade: o CI do site

- **Links quebrados** (internos e externos) falham o build.
- **Exemplos executáveis**: um job sobe um `rustployd` com `config.toml` de
  teste e roda os blocos marcados `# testado` (curl/JSON-RPC/manifesto/compose).
- **Referência gerada confere com o código**: se `comandos.md` mudou e a
  referência não foi regenerada, falha.
- **Frontmatter obrigatório** (`resumo`, `verificado_em`, `tipo` Diátaxis).
- **Lint de texto** (Vale ou similar): glossário, termos proibidos, tamanho
  de frase.
- **Lighthouse CI** com piso para SEO/performance/acessibilidade.
- **Screenshots reproduzíveis**: usar a receita Xvfb/Xephyr já existente
  (`rustploy_gui_visual_smoke_xvfb`) com dados de demonstração — nunca
  credenciais reais. Webui via Chrome headless (já usado nos testes).

## 10. Lacunas encontradas no que existe (a corrigir antes de documentar)

Achados enquanto mapeava; viram tarefas na Fase 0.

1. `README.md` do daemon ainda afirma "Não tem Web UI" e "Cliente: único é a
   GUI" — falso desde 2026-08-03.
2. `install.sh` instala um binário `rustploy` (hoje um shell script de uma
   linha) e aponta para a URL de release do repo antigo; **testar numa VPS
   limpa** antes de publicar o passo a passo.
3. **Paridade desigual**: 29 comandos não têm tela em nenhum cliente (ex.:
   autorizar acesso a banco compartilhado, restaurar backup de env,
   `IngressRoutes`/`IngressReconcile`, `PruneBuildCache`, `ManifestApply`). O
   site precisa dizer "só por API/MCP" nesses casos — e isso é um argumento a
   favor de destacar a trilha de MCP.
4. Funcionalidades **planejadas e não implementadas** (dependências + autostart,
   integração registry↔deploy, token com escopo): ficam fora do site ou marcadas
   "roadmap". Nunca descritas como existentes.
5. `docs/services.md` (37 KB) não tem cabeçalhos — é a fonte mais completa sobre
   serviços e precisa ser lida e reestruturada, não copiada.
6. Mensagens de UI estão em português; o site em pt-BR é coerente, mas o
   inglês exigirá decidir se as telas ganham i18n ou se os screenshots ficam em pt.
7. Não há **schema do protocolo** gerado (limitação já registrada no
   `AGENTS.md`); a referência de `Command` começa descritiva e melhora se um dia
   houver `schemars`.

## 11. Fases e entregas

| Fase | Entrega | Critério de pronto |
|---|---|---|
| **0 — Alicerce** | decisões das perguntas; esqueleto Astro/Starlight em `site/`; CI; tema; corrigir §10.1–2 | site vazio publicado em domínio de teste, Lighthouse ≥ 95 |
| **1 — Caminho feliz** | trilhas 0, 1, 2, 3 | um estranho instala e faz o primeiro deploy só com o site (teste com 1–2 pessoas) |
| **2 — MCP e agentes** | trilha 12 completa + `llms.txt` | alguém conecta o Claude Code a um servidor usando só a doc |
| **3 — Guias** | trilhas 4, 5, 6, 8 | cada serviço/recurso do `ServiceSpec` tem página |
| **4 — Referência gerada** | trilha 14 + 386 templates + paridade | regenera com `make docs-gen`; CI confere |
| **5 — Dados e operação** | trilhas 7, 9, 10, 13 | FAQ cobrindo os 20 problemas reais já registrados em `docs/licoes-aprendidas.md` |
| **6 — Interfaces e polimento** | trilha 11 com screenshots, comparativos, JSON-LD, OG images | |
| **7 — Inglês** | tradução das trilhas 0–3 e 12 primeiro | `hreflang` ativo |

A fase 2 vem antes dos guias de propósito: o MCP é o diferencial e é novo — se
o lançamento do site for gradual, é o que mais gera atenção e citação por IA.

## 12. Perguntas em aberto (preciso da sua decisão)

1. **Domínio** do site e do produto (define canonical, sitemap e a hospedagem).
2. **Idioma inicial**: só pt-BR (mais rápido) ou pt-BR + en desde o início?
   Recomendo pt-BR primeiro e en na fase 7.
3. **Onde mora**: `site/` no agregador (recomendado) ou um repo `rustploy-docs`?
4. **Framework**: confirma Astro + Starlight?
5. **Política para crawlers de IA**: liberar todos (recomendado, é o objetivo
   do GEO) ou restringir algum?
6. **Comparativos com Dokploy/Coolify** com nome e tabela: ok publicar?
7. **Instalação**: o fluxo oficial é `install.sh` via curl, o `.deb`, ou ambos?
   (a Fase 1 depende disso).
