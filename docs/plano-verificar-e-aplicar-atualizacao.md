# Plano: verificar a versão mais recente e atualizar pelo Settings

**Estado:** **proposto, não implementado.** Escrito em 2026-10-22. Nada aqui foi decidido: as
escolhas que dependem de você estão na seção 7. Antes de implementar, leia também
`docs/release.md` (como uma release nasce) e `docs/plano-tray-bandeja-e-ciclo-de-vida.md`.

## 1. O que você pediu, em uma frase

Um botão no Settings que pergunta "existe versão mais nova?" e, se existir, mostra um botão
**Atualizar**.

Dá para fazer. Mas "atualizar" quer dizer duas coisas bem diferentes aqui, e uma delas é muito mais
delicada do que a outra, por isso o plano é em fases.

## 2. Como as coisas funcionam hoje (o que já existe)

- **Uma release é uma tag `v*` no agregador.** O `release.yml` publica no GitHub Releases:
  `rustployd_<versão>_amd64.deb` e `_arm64.deb`, `rustploy-gui_<versão>_amd64.deb`,
  `rustploy-gui-windows-<tag>.zip` e `checksums.sha256`.
- **O `install.sh` já sabe achar a última release** pela API do GitHub e instalar por cima, mantendo
  o `config.toml` (`--force-confold`). Atualizar o daemon hoje é rodar de novo o comando de
  instalação na VPS (`docs`: `instalacao/atualizar`).
- **O daemon já diz a própria versão.** `Command::DaemonStatus` devolve `version`
  (`env!("CARGO_PKG_VERSION")`), uptime e contagem de serviços
  (`api/handlers/observability/daemon_status.rs`).
- **O daemon já fala com o GitHub** (`features/git_providers/github.rs`), então já existe cliente
  HTTP com TLS no processo.
- **O Settings tem 4 abas** na GUI (`views/home/settings/`: `web`, `git`, `iac`, `maintenance`) e uma
  tela equivalente na webui (`webui/.../settings.js`). Por convenção, feature de UI entra **nos dois**.
- **A GUI é outro programa, na máquina do usuário.** Ela tem a própria versão (`rustploy-gui` 0.1.15),
  instalada por `.deb`, por zip no Windows, ou por `cargo install`. Daemon e GUI nem sempre estão na
  mesma versão, então cada um tem o seu próprio estado de atualização.

## 3. O obstáculo que define o desenho

O daemon roda como o usuário **`rustploy`**, sem privilégios: o serviço tem `NoNewPrivileges=yes` e
`CapabilityBoundingSet=CAP_NET_BIND_SERVICE` (`packaging/rustployd.service`). Instalar um `.deb`
exige **root**. Ou seja:

> O daemon **não consegue** se atualizar sozinho. Um botão "Atualizar" que só chama o daemon não tem
> como instalar nada.

Isso não é detalhe: é de propósito (o daemon é a parte exposta à rede, e rodar nele como root seria
uma superfície de ataque grande). Existe, porém, um precedente no próprio projeto para "fazer algo
privilegiado sem dar root ao daemon": o **`rustployd-fw`**, um helper root minúsculo, ativado por
socket, que só aceita `allow`/`deny` de portas dentro de uma faixa. A atualização pode seguir o
mesmo molde.

## 4. Fase 1: verificar (somente leitura)

O que o botão **"Verificar atualizações"** faz. Não instala nada, não exige root, e já entrega a
maior parte do valor (saber que está desatualizado).

**Daemon.** Um comando novo, `Command::CheckUpdate`, que:

1. consulta `GET https://api.github.com/repos/antoniofernandodj/rustploy/releases/latest`;
2. compara a tag (`vX.Y.Z`) com `CARGO_PKG_VERSION` por semver (não por texto: `0.1.9` < `0.1.15`);
3. devolve `{ current, latest, update_available, release_url, published_at, checked_at }`;
4. guarda o resultado em memória por algumas horas (a API anônima do GitHub limita a **60
   requisições por hora por IP**, e uma VPS que várias pessoas abrem na UI estoura isso);
5. falha com mensagem clara quando a VPS não alcança o GitHub (rede fechada), sem travar a tela.

Por que o daemon consulta, e não a GUI: a pergunta é "o **daemon** está desatualizado?", e só ele
sabe a própria versão com certeza. Além disso a webui, a GUI e o agente MCP (`rpc`) ganham o mesmo
dado sem repetir a lógica.

**Telas (GUI + webui).** Uma aba nova **"Atualizações"** no Settings (ou uma seção na aba `web`, se
preferir menos abas; ver seção 7):

```
Versão instalada      0.1.6
Última versão         0.1.7        [Verificar atualizações]
Verificado em         22/10/2026 14:03

● Há uma versão nova.               [Atualizar]   (fase 2)
  Notas da release ↗
```

Na fase 1 o botão **Atualizar** ainda não instala: ele mostra o comando pronto para copiar
(`curl -fsSL …/install.sh | sudo sh`), com um botão "Copiar". Isso já resolve 80% do caso e não
carrega nenhum risco.

Segue a regra do projeto de botão travado até a resposta (`docs`: `botao_travado_ate_resposta`):
o "Verificar" entra na lista `busy_actions` da GUI e no equivalente da webui.

## 5. Fase 2: atualizar o daemon de verdade

Aqui mora o risco. Desenho proposto, seguindo o molde do `rustployd-fw`:

**Peças.**

- Um helper root novo, **`rustployd-update`**, ativado por socket (como o `-fw`). Superfície mínima:
  aceita **um** pedido, "instalar a versão X", e nada mais.
- O daemon, ao receber `Command::ApplyUpdate { version }`, só repassa o pedido ao helper pelo
  socket. Ele continua sem root.
- O helper: baixa o `.deb` **da release do repositório oficial** (URL montada por ele, nunca
  recebida do chamador), baixa o `checksums.sha256`, **confere o SHA-256** e só então roda
  `dpkg -i` com `--force-confold`. Não roda `install.sh` inteiro, porque o script pergunta de
  motor de containers e migra volumes; uma atualização normal não deve tocar nisso.

**O que acontece na hora.** O `dpkg` reinicia o serviço, então o daemon **cai e volta**. Os containers
dos serviços continuam rodando (são do Docker/Podman, não do processo), e o daemon retoma deploys
interrompidos ao subir (comportamento já documentado em `instalacao/atualizar`). A UI precisa
tratar isso como esperado: mostrar "Atualizando…", esperar a conexão voltar e comparar a versão
nova com a pedida. Se não voltar no prazo, mostrar o erro e o comando manual.

**Pontos que exigem cuidado.**

1. **Quem pode pedir.** Atualizar derruba a API por alguns segundos e troca o binário. Deve exigir
   confirmação explícita na UI ("Isso reinicia o daemon") e **não** ficar disponível ao agente
   MCP sem uma permissão própria; hoje o `rpc` expõe qualquer `Command`, então o novo comando
   precisa de uma trava.
2. **Verificação de integridade.** Sem checar o SHA-256 (e idealmente a assinatura), um helper root
   que instala o que baixa é um vetor de ataque. Fixar o repositório de origem, não aceitar URL do
   cliente.
3. **Sem volta fácil.** Um `.deb` instalado por cima não se desfaz sozinho. Guardar o `.deb` anterior
   em `/var/lib/rustploy/updates/` permite um `Command::RollbackUpdate` depois; na primeira versão,
   a mensagem de falha aponta o comando manual.
4. **Banco de dados.** Se a versão nova migra o schema, voltar atrás pode não servir. O docs já
   recomenda backup de `master.key` e `/var/lib/rustploy` antes de atualizações grandes; a UI deve
   repetir esse aviso.
5. **Deploy em andamento.** Hoje o daemon se recupera de deploy interrompido, mas é melhor o botão
   avisar quando há deploy na fila e pedir confirmação.
6. **Sem `sudo` na VPS ou sem systemd** (container, ambiente sem helper): a UI esconde o botão e fica
   só no comando para copiar.

## 6. Fase 3: atualizar a GUI

A GUI é um programa à parte, com versão própria e instalada de formas diferentes (`.deb`, zip no
Windows, `cargo install`). Ela roda **na máquina do usuário**, e é nela que a atualização acontece:
o daemon não participa (ele está em outra máquina e nem precisa estar conectado). A GUI se atualiza
**com a mesma arquitetura da fase 2**: consulta, verifica e entrega a instalação a quem tem
privilégio.

**Verificar.** A GUI consulta `releases/latest` no GitHub com o `fetch` global do Luau (o
`net/api.luau` já o usa no login) e compara com a própria versão. É a mesma lógica do daemon; o
comparador de semver deve ser um só, sem duas implementações divergindo. A conta é separada da do
daemon: a tela mostra "Daemon 0.1.6" e "App 0.1.15" cada um com o seu estado.

**Aplicar, por plataforma.** O que muda é quem tem permissão de escrever onde o app está instalado:

| Instalação | Quem instala | Como |
|---|---|---|
| Linux, `.deb` (`/usr/bin`) | root | o app pede elevação com `pkexec` (polkit mostra o diálogo de senha do sistema) e roda `dpkg -i` do `.deb` baixado e com SHA-256 conferido; não exige serviço permanente |
| Linux, `cargo install` (`~/.cargo/bin`) | o próprio usuário | sem root: baixa e substitui o binário, ou orienta `cargo install rustploy-gui` |
| Windows, zip portátil | o próprio usuário (pasta dele) | baixa o zip, verifica, **fecha o app** e um pequeno updater troca a pasta e reabre; o executável em uso não pode ser sobrescrito |

No Linux eu **não** criaria um helper root permanente como o `rustployd-fw`: esse padrão faz sentido
num servidor onde o daemon precisa agir várias vezes sem ninguém na frente. Num desktop há uma
pessoa presente, e `pkexec` dá o diálogo de autenticação nativo, sem deixar um processo root rodando.
O princípio é o mesmo (o programa comum não ganha root; o passo privilegiado é pequeno, fixa a origem
e confere o hash), só muda o mecanismo.

**Cuidados.**

- O app em execução é o que está sendo trocado. No Linux o `dpkg` substitui o binário e o app
  precisa **reiniciar** (a janela atual segue com o código antigo na memória). Mostrar "Reiniciar
  agora" ao final; no Windows o updater reabre sozinho.
- Mesma checagem da fase 2: origem fixa (repositório oficial), `checksums.sha256` conferido, nunca URL
  vinda de fora.
- A GUI tem a bandeja e o ciclo de vida próprios (`docs/plano-tray-bandeja-e-ciclo-de-vida.md`):
  confirmar que fechar para atualizar não deixa processo residente na bandeja.

**Pergunta técnica a verificar antes:** como o Luau da GUI descobre a versão do próprio app? Não
achei isso exposto hoje (o `env!("CARGO_PKG_VERSION")` não é usado em `rustploy-gui/src`). Falta só um
jeito de injetar a versão no contexto ao subir.

## 7. Decisões que dependem de você

1. **Escopo da primeira entrega.** Só **verificar + mostrar o comando** (fase 1, segura e rápida), ou
   já incluir o helper root (fase 2)? Recomendo começar pela fase 1: entrega o botão que você
   descreveu e deixa a decisão sobre privilégio para depois de usá-la.
2. **Daemon, GUI, ou os dois.** A fase 1 do daemon e a da GUI são independentes. Qual primeiro?
3. **Lugar na tela.** Aba nova "Atualizações" (mais visível) ou uma seção dentro da aba `web`
   (menos abas)?
4. **Canal.** Só a última release estável (`/releases/latest`), ou também *pre-releases*?
5. **Verificação automática.** Só ao clicar, ou também sozinha ao abrir o Settings (com o cache de
   algumas horas) e um selo discreto "atualização disponível"?
6. **VPS sem internet para o GitHub.** Aceitar que a verificação simplesmente diga "não consegui
   consultar" (proposta), ou prever um espelho próprio?

## 8. Plano de execução (fase 1, a que eu faria primeiro)

1. `rustploy-shared`: o tipo `UpdateInfo` e `Command::CheckUpdate` / `Response::UpdateInfo`; bump do
   `shared` e publicar (o daemon compila contra o crates.io; ver `docs/release.md`).
2. Daemon: handler `api/handlers/observability/check_update.rs` + cache em `AppState` + testes do
   comparador de semver (`0.1.9` < `0.1.15`, tag com e sem `v`, pre-release, resposta de erro do
   GitHub, limite de requisições).
3. GUI: aba/seção, handlers em `views/scripts/handlers/settings.luau`, ação na lista `busy_actions`,
   testes `*.test.luau` (estado antes e depois, erro de rede).
4. Webui: o mesmo em `settings.js`, com o mesmo texto.
5. Docs: `instalacao/atualizar` ganha "Verificar pelo Settings" (pt e en); o *Publish* de sempre
   (índice, bump, commit, release).

## 9. O que **não** está neste plano

- Atualização automática sem clique. Mudar o binário de um PaaS sem aviso é pior do que ficar uma
  versão atrás.
- Trocar o motor de containers (Docker/Podman) durante a atualização: segue sendo o `install.sh`.
- Atualizar os containers dos serviços do usuário (outra feature, e outro risco).
