# Prompt: depurar a infra pela API de agente

> Frase pronta para pedir a um agente (Claude Code, Codex…) que depure o rustploy
> pela **API de agente** da GUI. O manual da API é a "Parte 1" do `AGENTS.md`; o
> desenho, `docs/api-agente-no-gui.md`.

## Antes de usar

- O `rustploy-gui` precisa estar **aberto e logado** no servidor a investigar. A
  API empresta a sessão da janela, e o token local morre com o processo.
- Rode o agente **a partir do repositório do rustploy**: assim ele encontra o
  `AGENTS.md` e o `CLAUDE.md` sozinho.
- Com o app fechado, o `~/.local/share/rustploy/agent-api.json` não existe ou
  aponta para um token morto. Aí o agente deve tentar conectar pela receita 1 do
  manual ("Conectar sem ninguém na frente do app").

## O prompt

Troque `<nome>` pelo serviço com problema.

> Quero que você depure a infra do meu rustploy pela API de agente da GUI. Leia a
> "Parte 1 — Manual de Controle por Agente" do `AGENTS.md` do rustploy e comece
> pela descoberta: `cat ~/.local/share/rustploy/agent-api.json`, que traz a porta
> e o token local. Use só essa API HTTP em loopback, nunca fale direto com o
> daemon. Não faça nenhuma ação que altere estado (deploy, stop, restart, apagar)
> sem me perguntar antes. Comece listando projetos e serviços com seus estados.
> Depois investigue o serviço `<nome>` em duas frentes: o estado do deploy e a
> causa da última falha, e os logs de runtime. Se houver um domínio dando 502,
> siga a receita 6 do manual. No fim, me diga a causa provável e o que você faria
> para corrigir.

## Diagnóstico geral (sem um serviço em foco)

> Faça um diagnóstico geral e me dê um resumo dos serviços que não estão
> `running`, com a causa de cada um.

(Use junto com o prompt acima, no lugar do trecho sobre `<nome>`, ou depois dele.)

## Por que a frase de cautela

A API tem as **mesmas rotas de ação** que a GUI. Sem a instrução de não alterar
estado sem perguntar, um agente tentando "consertar" um 502 pode redeployar
sozinho. Remova essa frase só quando quiser, de propósito, que ele aja.
