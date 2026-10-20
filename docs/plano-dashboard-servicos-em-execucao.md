# Plano: dashboard "o que está rodando agora"

> Status: **implementado** (2026-10-20): item "Em execução" na sidebar (GUI e webui).

## O problema

Hoje a tela inicial é a lista de **Deployments** (histórico). Para saber "o que
está no ar, em qual branch, e como faço deploy/stop" é preciso entrar projeto
por projeto, serviço por serviço. O pedido é uma tela que responda isso de uma
vez.

## O que a tela mostra

- **Só projetos com pelo menos um serviço em execução.** Projeto vazio ou com
  tudo parado não aparece.
- Dentro de cada projeto, **só os serviços em execução** (estado `Running`).
- Cada projeto tem um atalho **"abrir projeto"**; cada serviço, **"abrir
  serviço"** (navegação para as telas que já existem).
- Cada serviço é um cartão resumido e **especializado pelo tipo**:

| Tipo | Como é detectado | O que o cartão destaca |
|---|---|---|
| App (Git) | `source = Git` | branch, URL do repositório, selo aquecido/frio |
| App (zip) | `source = Archive` | nome do zip, selo aquecido/frio |
| App (imagem) | `source = Registry` | imagem:tag |
| Banco de dados | `spec.db_kind` preenchido | engine (postgres, mongo…), porta, URL interna |
| Broker | `db_kind` de broker (kafka, rabbit…) | idem banco |
| Compose | `source = Compose` | nº de containers |

- Em todos: **atalhos Deploy e Stop**, CPU/mem, domínio(s), **webhook de
  deploy** (URL com botão copiar) e os **pré-deploy checks** (nomes dos jobs,
  na ordem em que rodam).
- Deploy e Stop reaproveitam os comandos que já existem (`DeployStart`,
  `ServiceStop`) e a regra "botão travado até a resposta" (`busy`).

## Onde o dado vem

Quase tudo já está no snapshot (que chega a cada 2 s pelo SSE): serviços com
status e containers, jobs (para resolver o nome de cada pré-deploy check),
métricas. As lacunas:

1. **Webhook**: hoje só existe `GetWebhookUrl` por serviço (uma chamada cada).
   Proposta: o snapshot passa a trazer `webhooks: { service_id: url }`, montado
   no daemon a partir da mesma função do comando. Uma ida só, sem N chamadas.
2. **Tipo "broker"**: preciso conferir quais valores `db_kind` assume hoje para
   classificar banco × broker; se não houver distinção, ela entra como lista
   fixa no `shared` (uma função só, que as duas telas usam — fonte única).

## Onde mora o código (nos dois clientes)

- **webui**: `screens/dashboard.js` (hoje é a tabela de Deployments) ganha a
  nova visão; a tabela de Deployments continua acessível.
- **GUI glacier**: novo `format/` + `.gvb` equivalente, ações no `handlers/`
  existentes (deploy/stop já têm handler).
- Regra do repo: feature de UI entra **nos dois**, e a lógica de "quem é
  banco/broker/app" e "o que mostrar" fica em uma função só no daemon/shared,
  as telas só exibem.

## Decisões tomadas (2026-10-20)

1. Item **novo** na sidebar ("Em execução"); Deployments segue como tela inicial.
2. Serviço parado de um projeto que tem outro rodando **some**.
3. Webhook **mascarado** por padrão (copiar sempre copia a URL inteira; "Mostrar" revela).

Implementado em `api/http/dashboard.rs` (snapshot.dashboard), `webui/screens/running.js`
e `rustploy-gui/views/{home/running.gvb,components/running_card.gvb,scripts/format/running.luau}`.

## Pergunta original

1. **A nova tela substitui a de Deployments como inicial**, ou entra como
   item novo da sidebar ("Painel") e Deployments continua como está?
   (Recomendo item novo + virar a tela inicial; Deployments segue um clique
   abaixo.)
2. **Serviço parado de um projeto que tem outro rodando**: some (como pedido)
   ou aparece apagado com o botão Deploy? (Pelo pedido: some.)
3. **Webhook visível por padrão** ou escondido atrás de um "mostrar"? A URL
   carrega o token, que dispara deploy para quem tiver. (Recomendo mascarar
   por padrão, com copiar e revelar.)

## Passos

1. Daemon/shared: `webhooks` no snapshot; classificação banco×broker.
2. webui: tela, filtros (só projetos/serviços rodando), cartões por tipo,
   atalhos, navegação.
3. GUI: o mesmo, com `glacier test` cobrindo o formatador.
4. Teste visual nos dois (Xvfb para a GUI) antes de dar por pronto.
