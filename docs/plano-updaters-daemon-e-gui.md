# Plano: updaters (daemon e GUI) — um instalador root minúsculo para cada app

**Estado:** **fases 0, 1, 2 e 3 implementadas, publicadas e provadas em ponta a ponta (2026-10-25, release v0.1.19). A fase 4 (Windows) segue adiada, por decisão.**
Escrito em 2026-10-25. As decisões tomadas estão na seção 11.

**Progresso.**
- **Fase 0 (assinatura):** feita no código: chave gerada (`keys/rustploy-update.pub`),
  `release.yml` cria rascunho, `tools/sign-release.sh`, `docs/release.md`. **Falta testar o fluxo do
  rascunho com uma tag descartável** (`gh release download/upload` contra um rascunho real nunca
  rodou).
- **Fase 1 (verificar):** feita: `rustploy-update-core` (comparador semver) e `UpdateInfo`/`CheckUpdate`
  no `rustploy-shared` 0.9.0; no daemon, `platform/https_client.rs` (hyper + tokio-rustls +
  webpki-roots, sem reqwest), `features/update/`, `[update]` na config e o handler; aba
  "Atualizações" na GUI e na webui. Testada contra o GitHub de verdade (respondeu `v0.1.15`), com
  cache, repositório inexistente, repositório inválido e verificação desligada.
- **Fase 2 (updater do daemon), 2026-10-25:** implementada. `update-core` 0.2.0 (assinatura, dpkg, pacote, staging, outcome),
  `rustployd-update` (helper root por socket, units, `postinst`), `ApplyUpdate`/`UpdateStatus`/`UpdateRollback` no daemon
  com snapshot do banco, download com redirecionamento só para hosts do GitHub, trava do agente MCP; botões nas duas UIs.
  Testada com 304 testes do daemon, 19 do helper, 40 do `update-core` e um **teste de ponta a ponta com systemd, dpkg e socket
  reais** (`tests/e2e-updater/run.sh`, 11 cenários). **Ainda sem prova em produção:** o fluxo completo com download do GitHub
  só pode ser exercitado quando houver duas releases com o helper (a primeira já nasce com ele).
- **Fase 3 (updater do app, Linux), 2026-10-25:** implementada. `update-core` 0.3.0 passou a conter o instalador, o acesso ao
  sistema e a escolha de arquivos da release (o daemon e o app usam o mesmo código). App: `rustploy-gui-update` (helper root via
  `pkexec`, sem processo residente), política do polkit, ponte nativa `app_update` e a seção "App desktop" na aba. Testada com
  `tests/e2e-updater-gui/run.sh` (pkexec e polkit reais, 11 cenários) além dos testes unitários. **Windows segue adiado (fase 4).**
  **Não testado:** o diálogo de senha real num desktop (o teste usa uma regra de polkit que autoriza sem senha) e o fluxo de ponta
  a ponta com download do GitHub, que só uma release com o helper pode exercitar.
- **Prova real (2026-10-25):** o daemon da v0.1.17 se atualizou sozinho para a v0.1.18 pelo `ApplyUpdate` (download do GitHub,
  assinatura de produção, snapshot, reinício, rollback, trava do agente MCP: 17/17), e o da v0.1.18 para a v0.1.19 (19/19). O
  primeiro teste achou um defeito de projeto: a verificação comparava a **tag** da release com a versão do programa, numerações
  independentes, e depois de atualizar ainda dizia "há versão nova". Corrigido em `update-core` 0.3.2 (`package_version` lê a versão
  do nome do `.deb`); o segundo teste confirma que depois de atualizar o daemon se vê em dia.
- **Pendências conhecidas:** o diálogo de senha **real** do `pkexec` num desktop (os testes usam uma regra de polkit que autoriza
  sem senha); a atualização do app com o download do GitHub ponta a ponta (a ponte nativa e o helper foram testados, mas o fluxo
  clicado numa janela real não); o selo "versão nova" fora da aba; chaves de assinatura de
  reserva para rotação. Windows: fase 4.
- **Publish da fase 1 (2026-10-25):** `rustploy-update-core` 0.1.0 e `rustploy-shared` 0.9.0 publicados; daemon 0.1.10 e GUI 0.1.20; docs pt/en atualizadas.
- **Antes (histórico) faltava o *Publish*:** publicar `rustploy-update-core` 0.1.0 e `rustploy-shared`
  0.9.0 no crates.io *antes* do daemon (ele depende das duas), atualizar o `Cargo.lock` do daemon, bump
  do daemon e da GUI, commit/push dos submódulos e do agregador, release e docs. O banner de "versão
  nova" (selo discreto fora da aba) ficou de fora desta entrega.

**Histórico.** Este documento absorveu o antigo `plano-verificar-e-aplicar-atualizacao.md`
(2026-10-22, removido; está no git). Aquele plano trazia a verificação de versão (só leitura, sem
root, hoje a fase 1 aqui) e um esboço de aplicar a atualização. O desenho de aplicar foi trocado pelo
seu (um updater por app, com root, que só instala), levado mais longe em um ponto: **o updater não
baixa nada**, só confere e instala.

---

## 1. A ideia, nas suas palavras e nas minhas

Você descreveu:

> um updater para o daemon e um updater para o GUI, com acesso root que só tenha a funcionalidade de
> instalar no app correspondente o que vem de algum lugar. O app a ser atualizado checa se há ou não a
> atualização, requisita ao updater correspondente e instala.

É exatamente o desenho que proponho, com um refinamento que o torna bem mais seguro:

```
 ┌───────────────┐  1. consulta "tem versão nova?"   ┌──────────────────────┐
 │ app (daemon   │ ────────────────────────────────▶ │ GitHub Releases      │
 │  ou GUI)      │ ◀──────────────────────────────── │ (ou outro lugar)     │
 │ SEM root      │  2. baixa o .deb + a assinatura   └──────────────────────┘
 │               │
 │               │  3. "instale este arquivo"        ┌──────────────────────┐
 │               │ ────────────────────────────────▶ │ updater (ROOT)       │
 │               │                                   │  • confere assinatura│
 │               │ ◀──────────────────────────────── │  • confere nome/versão│
 └───────────────┘  4. resultado                     │  • roda dpkg -i      │
                                                     └──────────────────────┘
```

**O refinamento:** quem fala com a internet é o **app** (sem privilégio). O **updater root não abre
nenhuma conexão de rede**: recebe um arquivo, prova que ele é legítimo e instala. Isso é o que o
"instalar o que vem de algum lugar" pede de verdade, e traz duas vantagens:

1. O processo root não tem cliente HTTP, TLS, DNS, nem parser de JSON da internet. Quase tudo que
   costuma dar errado num updater (redirecionamento malicioso, resposta gigante, certificado) acontece
   no lado sem privilégio.
2. A origem deixa de importar. GitHub, um espelho seu, um pendrive: o updater só instala o que estiver
   **assinado por você**. Isso responde sua dúvida "possivelmente (ou não) do GitHub": pode ser do
   GitHub hoje e de outro lugar amanhã, sem mexer na parte root.

## 2. O que já existe e que reaproveitamos

- **`rustployd-fw`** (`crates/daemon/src/bin/rustployd-fw.rs`): helper root do firewall, ativado por
  socket (`/run/rustploy/fw.sock`, `root:rustploy 0660`), protocolo de uma linha JSON, superfície
  mínima. O updater do daemon segue **o mesmo molde**, então a forma de empacotar, ativar e testar já
  está resolvida no projeto.
- **A release** (`release.yml`) já gera `rustployd_<v>_amd64.deb`/`_arm64.deb`,
  `rustploy-gui_<v>_amd64.deb`, o zip do Windows e `checksums.sha256`, e o `install.sh` já acha a última
  release pela API do GitHub e já confere SHA-256.
- **O daemon** já responde a própria versão (`Command::DaemonStatus`) e já tem cliente HTTP para o
  GitHub (`features/git_providers/github.rs`).
- **O `install.ps1`** instala a GUI do Windows **por usuário**, em `%LOCALAPPDATA%\Rustploy`, sem
  administrador.

O que **não** existe: nenhuma assinatura. Hoje só há `checksums.sha256`, que baixa da mesma release que
o arquivo; se alguém trocar a release, troca os dois. Isso é a peça nova mais importante deste plano
(seção 3).

## 3. A peça que sustenta tudo: assinatura dos pacotes

Um updater root que instala "o que vem de algum lugar" só é seguro se conseguir provar **quem
produziu** o arquivo. Soma de verificação (SHA-256) não prova isso: só prova que o arquivo não se
corrompeu no caminho. Por isso:

- Geramos **um par de chaves ed25519** (formato minisign) **na sua máquina**. A **privada** é
  protegida por senha (o `minisign -G` pergunta) e **nunca vai para o GitHub nem para o CI**; ela tem
  uma cópia offline (gerenciador de senhas e um pendrive). A **pública** é embutida no código dos
  updaters (e publicada no repo e nas docs).
- **Quem assina é você, à mão, a cada release.** O `release.yml` compila e anexa os pacotes a uma
  **release em rascunho** (*draft*); você baixa os `.deb`, assina no seu computador
  (`minisign -S`, gerando `<arquivo>.minisig`), sobe os `.minisig` na mesma release e só então a
  **publica**. O comentário confiável da assinatura carrega
  `package=rustployd version=0.2.0 arch=amd64`, que entra na assinatura e não pode ser alterado.
- Por que não assinar no CI: quem invadisse a sua conta do GitHub ou o repositório assinaria o que
  quisesse. Com a chave só na sua máquina, isso não basta para colocar código root em ninguém. O custo
  é um passo manual por release.
- O updater só instala se a assinatura bater com a chave pública embutida.
- **Rotação de chave:** o updater aceita até **duas** chaves públicas (a atual e a próxima). Para
  trocar, publica-se uma versão que já confia na nova; depois a antiga sai. Sem isso, perder a chave
  privada deixaria todas as instalações sem como atualizar.
- **Se a chave privada vazar**, é o pior cenário: quem a tiver pode publicar um pacote que a frota
  inteira instala como root. Por isso a privada tem senha, não sai da sua máquina e tem backup offline.

Biblioteca: `minisign-verify` (só verifica, sem dependências pesadas), no lado root. A assinatura em si
é feita por você com a ferramenta `minisign`.

**Release em rascunho e quem a vê.** Enquanto a release é rascunho, a API `releases/latest` não a
lista, então nenhuma instalação a enxerga antes de assinada. Uma release publicada **sem** `.minisig`
simplesmente não é oferecida pelo updater (o `install.sh` manual continua funcionando).

## 4. O updater do daemon: `rustployd-update`

**Forma.** Um binário novo ao lado do `rustployd-fw`, instalado em `/usr/libexec/rustployd-update`
pelo mesmo `.deb` do daemon, ativado por socket do systemd:
`/run/rustploy/update.sock`, `root:rustploy`, modo `0660`. Só root e o daemon (grupo `rustploy`)
conectam. O serviço atende **um pedido e sai** (não fica residente); o systemd o reativa no próximo.

**Protocolo.** Uma linha JSON por pedido, como o `-fw`:

```
→ {"op":"install","deb":"/var/lib/rustploy/updates/rustployd_0.2.0_amd64.deb",
   "sig":"/var/lib/rustploy/updates/rustployd_0.2.0_amd64.deb.minisig"}
← {"ok":true,"accepted":true}            // verificação passou, instalando
← {"ok":false,"error":"assinatura inválida"}

→ {"op":"status"}
← {"ok":true,"state":"idle","last":{"version":"0.2.0","result":"ok","at":"…"}}
```

Só existem **dois verbos**: `install` e `status`. Nenhum campo vira comando de shell (os `Command` são
montados por argv).

**O que o updater faz, nesta ordem (qualquer falha para tudo):**

1. Pega um **lock** exclusivo (`flock`): uma instalação por vez.
2. Abre o `.deb` indicado **sem seguir links simbólicos**, exige que seja arquivo regular dentro de
   `/var/lib/rustploy/updates/`, com limite de tamanho (ex.: 300 MB).
3. **Copia para um diretório só do root** (`/var/lib/rustploy-update/staging/`). Daqui em diante só
   se lê a cópia: o daemon não consegue trocar o arquivo depois da conferência (evita o TOCTOU,
   "conferiu um, instalou outro").
4. **Confere a assinatura** da cópia. Antes disso, **nada** é interpretado: o `dpkg-deb` só toca num
   arquivo já provado legítimo.
5. Lê com `dpkg-deb -f` o `Package`, `Version` e `Architecture` e exige que **batam** com o comentário
   assinado.
6. **`Package` precisa ser `rustployd`** (nome fixo no código do updater). Um `.deb` válido e assinado
   de outro pacote, como o `rustploy-gui`, é recusado: este updater só mexe no daemon.
7. A arquitetura precisa ser a da máquina.
8. **Sem downgrade:** `dpkg --compare-versions <nova> gt <instalada>`. Voltar versão é uma operação
   separada e explícita (seção 8), nunca um efeito colateral. Isso também derruba o ataque de repetir um
   pacote antigo, legítimo e assinado, mas com falha de segurança conhecida.
9. Faz **backup dos binários atuais** (seção 8) e roda
   `DEBIAN_FRONTEND=noninteractive dpkg -i --force-confold <cópia>`. Usa `dpkg`, **não** `apt`, de
   propósito: o `apt` resolveria dependências baixando da rede, como root, que é o que queremos evitar.
   Se a versão nova exigir dependência que falta, o `dpkg` falha e a UI mostra o erro e o comando manual.
10. Espera o serviço estabilizar e registra o resultado em `/var/lib/rustploy-update/last.json`.

**Hardening do serviço.** Diferente do `-fw`, não dá para usar `ProtectSystem=strict`: o `dpkg` escreve
em todo o sistema. Então o hardening é por **superfície**: sem rede (`PrivateNetwork=yes`, que é
possível justamente porque o updater não baixa nada), `ProtectHome=yes`, `PrivateTmp=yes`,
`NoNewPrivileges` não se aplica a quem precisa rodar `dpkg`. A garantia de verdade vem da assinatura e
das conferências acima, não do sandbox.

**Armadilha de ciclo de vida (precisa de cuidado na implementação).** O `postinst` do daemon hoje faz
`restart rustployd` e `restart rustployd-fw.socket`. Se fizesse o mesmo com o socket do updater, e o
service dele tivesse `Requires=` no socket, o systemd **mataria o updater no meio do `dpkg`**. Por isso:
(a) o service do updater **não** declara `Requires=` no socket; (b) o `postinst` só **habilita** o socket
do updater e nunca reinicia o service enquanto há instalação em curso (o updater define
`RUSTPLOY_UPDATING=1` e o `postinst` respeita). Como o service sai ao fim de cada pedido, o próximo
pedido já sobe o binário novo.

## 5. O updater da GUI: `rustploy-gui-update`

A GUI tem uma diferença estrutural: ela roda **na máquina de uma pessoa**, não num servidor. Dois
mecanismos são possíveis; a verificação é **idêntica** nos dois, só muda como o root é obtido.

**Escolhido: `pkexec`, sem processo residente.** O `.deb` da GUI instala
`/usr/libexec/rustploy-gui-update` e uma política do polkit
(`/usr/share/polkit-1/actions/br.com.rustploy.gui-update.policy`). Quando o usuário clica em
"Atualizar", a GUI baixa o arquivo e executa `pkexec /usr/libexec/rustploy-gui-update install …`; o
sistema mostra o diálogo nativo de senha, o helper roda como root **uma vez** e sai. Nada fica rodando
como root entre uma atualização e outra.

- Política: `auth_admin_keep` (pede a senha, lembra por alguns minutos). Instalar software como root
  merece um prompt.
- Sem agente polkit (sessão sem desktop completo): a GUI cai no plano B e mostra o comando manual.

**Descartada: serviço residente por socket, igual ao do daemon.** Mais parecida com o que você descreveu
e com um fluxo sem diálogo de senha. Custo: um processo root presente em **todo desktop** com a GUI
instalada, e o socket precisa aceitar qualquer usuário local (não existe um grupo `rustploy` no
desktop). A verificação por assinatura continua impedindo que um usuário local instale algo que não seja
seu, mas ele poderia disparar a instalação de uma versão legítima e mais nova sem a senha de admin.
Funciona, só é uma superfície maior sem ganho real para um app de desktop, onde há uma pessoa presente
para digitar a senha. Por isso recomendo a A.

**Conferências:** as mesmas dos passos 1–9 da seção 4, com `Package` fixo em `rustploy-gui`. O código é
o mesmo (seção 6); só o nome do pacote e o jeito de ser chamado mudam.

**Depois de instalar.** O `dpkg` troca o binário no disco, mas a janela aberta continua rodando o código
antigo na memória. A GUI mostra "Reiniciar agora" e reexecuta `/usr/bin/rustploy-gui`. É preciso checar
com o `docs/plano-tray-bandeja-e-ciclo-de-vida.md` que fechar para atualizar não deixa processo
residente na bandeja.

**Windows** (sem root, instalado em `%LOCALAPPDATA%\Rustploy`): não há o que elevar. A GUI baixa o zip
e a assinatura, e um **mini executável** (`rustploy-update.exe`, copiado para o temp para não ser a
própria pasta em uso) espera a GUI fechar, **verifica a mesma assinatura**, troca a pasta e reabre o app.
Mesma chave, mesmo código de verificação; só o "instalar" é "trocar arquivos" em vez de `dpkg`.

**`cargo install` (`~/.cargo/bin`):** fora do escopo do updater; a tela mostra
`cargo install rustploy-gui --force`. O app descobre como foi instalado olhando se o próprio binário
está em `/usr/bin` (`.deb`), em `%LOCALAPPDATA%\Rustploy` (zip) ou em outro lugar (só avisa).

## 6. Onde mora o código (e por que não duplicar a lógica de segurança)

A parte que decide "instalar ou não" é delicada, e **não pode existir em duas versões** divergindo. Mas
daemon, GUI e shared são três repositórios, e cada `.deb` é compilado no CI do seu pacote. Proposta:

- **`rustploy-shared`**: nasce um crate irmão **`rustploy-update-core`** (biblioteca publicada no
  crates.io, como o `shared`), com tudo que é comum e puro: verificar assinatura minisign, parsear e
  comparar versões (semver/dpkg, **um** comparador só, serve também à fase 1), validar nomes de pacote
  e caminhos, montar o `dpkg -i`, escrever o `last.json`. Fica testável sem root.
- **Daemon:** `src/bin/rustployd-update.rs`, fino, ao lado do `rustployd-fw.rs` (a casca de socket e o
  `package = "rustployd"`).
- **GUI:** `src/bin/rustploy-gui-update.rs`, fino (a casca de `pkexec` e `package = "rustploy-gui"`).
- Os dois binários finos **não decidem nada**: chamam o `update-core`. Um bug de segurança é corrigido
  em um lugar só.

Alternativa mais simples (menos arquivos), com um custo: um único binário `rustploy-updater` com
subcomandos, publicado por si só. Mas aí cada `.deb` precisaria conseguir embuti-lo vindo de outro repo,
o que complica o `release.yml`. Por isso preferi a opção acima, que segue o padrão `rustployd-fw`.

## 7. O lado do app (sem privilégio): checar, baixar, pedir

**Daemon: verificar (fase 1, somente leitura, sem root).** `Command::CheckUpdate`:

1. consulta `GET https://api.github.com/repos/<repo>/releases/latest` com o cliente de
   `features/git_providers/github.rs`;
2. compara a tag (`vX.Y.Z`) com `CARGO_PKG_VERSION` por **semver**, não por texto (`0.1.9` < `0.1.15`),
   com o comparador único do `update-core`;
3. devolve `{ current, latest, update_available, release_url, published_at, checked_at }`;
4. guarda o resultado em memória por algumas horas: a API anônima do GitHub limita a **60 requisições
   por hora por IP**, e uma VPS que várias pessoas abrem na UI estoura isso;
5. falha com mensagem clara quando a VPS não alcança o GitHub, sem travar a tela.

Quem consulta é o daemon, e não a GUI, porque a pergunta é "o **daemon** está desatualizado?" e só ele
sabe a própria versão com certeza; a webui, a GUI e o agente MCP ganham o mesmo dado sem repetir lógica.
Enquanto a fase 2 não existe, o botão "Atualizar" só mostra o comando pronto para copiar
(`curl -fsSL …/install.sh | sudo sh`). Testes: `0.1.9` < `0.1.15`, tag com e sem `v`, pre-release,
erro do GitHub, limite de requisições. O "Verificar" entra em `busy_actions` (GUI) e no equivalente da
webui.

**Daemon: aplicar (fase 2).** O que entra a mais:

- `Command::ApplyUpdate { version }`: baixa **o `.deb` e o `.minisig` da própria release oficial**
  (URL montada pelo daemon a partir de `repo` fixo na config, **nunca recebida da UI**) para
  `/var/lib/rustploy/updates/` com teto de tamanho, confere espaço em disco, e então chama o updater
  pelo socket. O daemon **não precisa confiar no que baixou**: o updater verifica de novo.
- Máquina de estados visível na UI: `Ocioso → Verificando → Disponível → Baixando → Instalando →
  (daemon reinicia) → Concluído | Falhou`.
- **Quando o daemon cai e volta:** o `dpkg` reinicia o serviço, a UI perde a conexão. A UI trata isso
  como esperado ("Atualizando…"), refaz `DaemonStatus` até a versão bater com a pedida, ou mostra o
  erro com o motivo (lido de `last.json`, que o daemon novo consegue ler ao subir). Os containers dos
  serviços seguem rodando (são do Docker/Podman) e o daemon retoma deploys interrompidos ao subir.
- **Checagem periódica** configurável (seção 10) e um selo discreto "atualização disponível".

**GUI.** A mesma lógica no próprio app, em Rust (não em Luau): baixar um `.deb` de dezenas de MB não é
trabalho para o `fetch` do Luau, e a versão do app (`env!("CARGO_PKG_VERSION")`) ainda não é exposta ao
Luau, então entra junto: o Rust injeta `app_version` e o estado da atualização no contexto das telas.
O Luau só desenha e dispara a ação (que entra na lista `busy_actions`, regra do botão travado).

**Telas, GUI + webui (os dois, sempre).** Uma seção "Atualizações" no Settings com **duas linhas
independentes**, porque daemon e GUI quase nunca estão na mesma versão:

```
Daemon   0.1.7  →  0.1.9    [Atualizar daemon]   notas ↗
App      0.1.19 →  0.1.20   [Atualizar app]      notas ↗
Verificado em 25/10/2026 14:03                    [Verificar agora]
```

A webui só mostra o bloco do daemon (ela é servida pelo próprio daemon e não tem "app" para atualizar).

## 8. Falhas, rollback e o que ainda pode dar errado

- **Rollback automático dos binários.** Antes do `dpkg -i` o updater copia `/usr/bin/rustployd` e
  `/usr/libexec/rustployd-fw` (ou o binário da GUI) para `/var/lib/rustploy-update/backup/`. Se depois
  de instalar o serviço não ficar estável (ex.: `rustployd` não fica `active` por 30 s, ou entra em
  loop de `Restart=`), o updater restaura os binários antigos e reinicia. É um rollback de
  **binário**, não do pacote inteiro.
- **Limite honesto:** se a versão nova **migrou o schema do banco**, voltar o binário velho pode não
  funcionar com o banco novo. Por isso a UI avisa antes ("faça backup de `master.key` e
  `/var/lib/rustploy`") e o daemon tira um **snapshot do banco** logo antes de pedir a instalação
  (entra na fase 2). Rollback manual pedido pelo usuário: `op: "rollback"` no updater, que restaura o
  backup e é o **único** caminho que permite versão menor.
- **Deploy na fila ou em andamento:** o botão avisa e pede confirmação; hoje o daemon se recupera, mas
  é melhor não surpreender.
- **Falha de dependência no `dpkg`:** o erro vai para a UI com o comando manual
  (`curl … install.sh | sudo sh`).
- **Quem pode pedir:** exigir confirmação explícita na UI ("isto reinicia o daemon") e **trancar o
  `ApplyUpdate` para o agente MCP**: o `rpc` hoje expõe qualquer `Command`; o novo precisa de uma trava
  em config (`[update] allow_agent_apply = false` por padrão).
- **Sem systemd ou sem o helper** (container, instalação por `cargo`): a UI detecta que o socket não
  existe e esconde o botão, mantendo só o comando para copiar.
- **Primeira instalação:** as versões **anteriores** a esta feature não têm updater. A primeira
  atualização que o traz é feita uma vez pelo `install.sh` (que já cuida de tudo). Dali em diante é pelo
  botão. O `install.sh` precisa também ser capaz de baixar e **verificar a assinatura** quando o
  `minisign` existir na máquina (melhoria opcional).
- **Chave de assinatura perdida:** sem ela, nenhum release novo é aceito pelas instalações existentes.
  Mitigação: segunda chave embutida desde o início (seção 3) e backup offline.

## 9. Plano de execução (em fases, cada uma entregável)

**Fase 0 — Assinatura (pré-requisito de tudo).**
1. Gerar o par de chaves ed25519 (você, na sua máquina, com senha). Guardar a privada e a cópia
   offline; me passar só a pública.
2. `release.yml`: passa a criar a release em **rascunho** (`draft: true`), sem assinar nada. Um script
   (`tools/sign-release.sh`) baixa os `.deb` e o zip do Windows do rascunho, assina com a sua chave,
   sobe os `.minisig` e publica a release. Documentar o fluxo novo em `docs/release.md`.
3. Publicar uma release de teste e **verificar a assinatura à mão** (`minisign -V`).

**Fase 1 — Verificar (o plano antigo, sem root).** `Command::CheckUpdate`, comparador único, cache, as
duas linhas na tela (GUI e webui), `busy_actions`, testes. Já entrega o aviso e o comando para copiar.

**Fase 2 — `rustploy-update-core` + updater do daemon.**
1. `update-core` com testes **sem root**: assinatura válida/inválida/adulterada, comentário que não
   bate, pacote errado, arquitetura errada, downgrade, link simbólico, arquivo grande demais, caminho
   fora do diretório.
2. `rustployd-update` + units `.socket`/`.service`, `postinst` (habilitar sem reiniciar durante o
   update).
3. `Command::ApplyUpdate` e a máquina de estados; teste ponta a ponta em container `debian:12` com
   systemd real (cobre o ponto "ainda sem teste real" do `release.md`).
4. Botão "Atualizar daemon" nas duas UIs, com confirmação.
5. **Rollback e snapshot do banco** (seção 8): backup dos binários antes do `dpkg -i`, restauração
   automática se o serviço não estabilizar, `op: "rollback"` manual e snapshot do banco antes de
   pedir a instalação. Testar com um pacote de teste que quebra de propósito.

**Fase 3 — Updater da GUI (Linux).** `rustploy-gui-update`, política polkit, download no Rust, versão
do app no contexto, reinício do app, checagem do ciclo de vida da bandeja.

**Fase 4 — Windows (ADIADA).** `rustploy-update.exe` com a verificação compartilhada, troca da pasta e
reabertura. Fica para depois: o `install.ps1` manual cobre por enquanto. O zip do Windows já sai
assinado na fase 0, então a base fica pronta.

Cada fase termina com o checklist de *Publish* do `AGENTS.md` (índice, bump, commit/push, crates.io,
release, docs). A documentação (`rustploy-docs`) ganha uma página "Atualizar pelo Settings" e a seção
de segurança do modelo de assinatura.

## 10. Configuração nova no daemon (`config.toml`)

```toml
[update]
enabled = true                 # false: nem consulta
repo = "antoniofernandodj/rustploy"   # origem fixa; a UI não escolhe
channel = "stable"             # "stable" = só /releases/latest; "beta" inclui pre-release
check_interval_hours = 6       # 0 = só ao clicar
allow_agent_apply = false      # o agente MCP não pode aplicar
```

O parse da config é tudo-ou-nada (regra do `AGENTS.md`), então a seção é opcional e tem padrões.

## 11. Decisões tomadas

Respondidas em 2026-10-25; o plano acima já reflete todas elas.

1. **Assinatura:** minisign (ed25519), **chave só na sua máquina**, com senha e backup offline. Quem
   assina é você, à mão, numa release em rascunho; o CI não tem a chave (seção 3).
2. **Origem:** GitHub Releases, fixo. Trocar depois não mexe na parte root.
3. **GUI no Linux:** `pkexec` + polkit, sem processo root residente.
4. **Senha do `pkexec`:** pedir normalmente (`auth_admin_keep`).
5. **Código comum:** crate `rustploy-update-core` no repo do `shared`.
6. **Primeira entrega:** fases 0 e 1 (assinar e verificar, sem root); a fase 2 vem em seguida.
7. **Ordem:** daemon (fase 2) antes da GUI (fase 3).
8. **Canal:** só estável (`channel = "stable"`).
9. **Windows:** adiado (fase 4); `install.ps1` manual por enquanto.
10. **Rollback e snapshot do banco:** entram na fase 2.

## 12. O que **não** está neste plano

- Atualização automática sem clique. Trocar o binário de um PaaS sem aviso é pior do que ficar uma
  versão atrás.
- Trocar o motor de containers (Docker/Podman): segue sendo o `install.sh`.
- Atualizar os containers dos serviços dos usuários (outra feature, outro risco).
- macOS e outras distros além de Debian/Ubuntu (o instalador só fala `.deb` e zip do Windows).
- Atualizar o próprio mecanismo de assinatura além da rotação de chave (seção 3).
