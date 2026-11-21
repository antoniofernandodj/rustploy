---
name: entrega-autonoma
description: Constrói uma funcionalidade nova do Rustploy de ponta a ponta SEM supervisão do dono — plano em docs/, implementação no daemon e nos DOIS clientes (GUI e webui), testes de verdade, publish completo assinado, docs pt/en no ar, índice, agregador e instalação local do app e do daemon por comando. Use quando o dono disser /entrega-autonoma, "faz tudo sozinho", "não vou poder supervisionar", "do plano ao publish" ou pedir uma feature com autonomia total.
---

# Entrega autônoma

O dono vai estar fora. Invocar esta skill **é** a autorização para tudo abaixo, nesta entrega:
commitar e dar push em todos os repos, `cargo publish`, criar tag e release, **assinar com o
`minisign.pass`** (método do shim, ver memória `feedback_assinar_release_com_minisign_pass`),
fazer deploy das docs e atualizar o app e o daemon **desta máquina**. Não vale para servidores
remotos de produção (Chiquitos, Itemize): neles, só leitura pelo MCP, salvo se o pedido disser o contrário.

**Não pare para perguntar.** Decisão de desenho que for do dono: escolha a opção mais conservadora
(a que não apaga nada e mantém o que existe funcionando), registre a escolha e o porquê no plano, e
siga. Pare só se continuar for destruir dados, ou se uma verificação falhar e você não souber corrigir.

## O pipeline, na ordem

1. **Estado limpo.** `git status` e `git fetch` nos dois sentidos em todos os repos (agregador,
   submódulos e `~/Development/rust/glacier-ui` se for mexer nele). `df -h /`: abaixo de ~15 GB livres,
   `cargo clean` no que não vai ser compilado (o disco já encheu no meio de um publish).
2. **Plano** em `docs/plano-<assunto>.md`, em linguagem simples, com contexto e porquês (o dono não
   gosta de plano condensado): o problema, o modelo, a tela nos dois clientes, a migração de dados,
   as fases, o que entra **nesta** entrega e o que fica para depois. Cabeçalho com o status.
3. **Implementação.** Leia antes o `AGENTS.md` (Convenções) e o `docs/indice/INDEX.md`.
   - Daemon (protocolo, handler, store) **e** GUI (`.gv`/`.gss`/Luau) **e** webui (`webui/`): toda
     feature de interface entra nos dois clientes.
   - **Toda ação destrutiva pede confirmação**, nos dois clientes (apagar, remover, esquecer, parar,
     limpar, revogar, sobrescrever). Ação nova na GUI entra na lista `busy_actions`.
   - **Modelo de dados:** campo novo com `#[serde(default)]`; mudança de formato no YAML de infra ou no
     SQLite exige migração no boot (`store/infra/migrate.rs`, migrações do sqlx) que não perde nada e é
     testada com um estado antigo de verdade. Diga no plano se a versão anterior consegue ler o novo.
4. **Testes que provam.** `make lint` e `make test` verdes; teste unitário do caso novo; quando a
   mudança for de comportamento, teste de ponta a ponta num **daemon de teste isolado** (memória
   `rustployd_sem_cli_sobe_o_daemon`) e, para tela, a GUI num Xvfb (memória
   `rustploy_gui_visual_smoke_xvfb`) com captura. Rode o mesmo roteiro no código antigo quando der,
   para provar que o teste pega o defeito. `glacier test` na GUI. Mexeu no updater: os dois `e2e-updater`.
5. **Publish completo**: a seção "Publish" do `AGENTS.md` e o `docs/release.md`, sem pular nada
   (bump só do que mudou, commit e push antes do `cargo publish`, CI sem tag verde antes da tag,
   assinatura, conferência da release e do `install.sh` num `debian:12`, `install.ps1` até o SHA-256).
6. **Docs** pt e en no `rustploy-docs` (capturas com host e segredos borrados), `npm run build`,
   `check-links` com 0 quebrados, commit, push, `npx wrangler deploy` e `curl` nas páginas novas.
7. **Índice e agregador**: `make index`, commit dos ponteiros e do `docs/indice/`, push.
8. **Instalar nesta máquina, por comando** (sem pedir senha), depois que a release estiver publicada:
   - daemon local: ferramenta MCP `rustploy-localhost` → `rpc` com `ApplyUpdate`; espere o daemon
     voltar e confira a versão com `status`;
   - app desktop: `rpc` com `ApplyAppUpdate` no mesmo daemon local (o helper root dele instala o
     `rustploy-gui` assinado); confira com `dpkg -s rustploy-gui`. O app aberto continua na versão
     antiga até ser reaberto: avise.
9. **Notas** da versão no `docs/release.md` (o que foi verificado e o que **não** foi), commit e push.
10. **Relatório final** ao dono, curto: o que entrou, o que foi verificado de verdade, o que não deu
    para verificar, e qualquer decisão tomada no lugar dele.
