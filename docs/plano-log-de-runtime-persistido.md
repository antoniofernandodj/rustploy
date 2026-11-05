# Plano: persistir o log de runtime dos containers

> **Estado: PROJETADO (2026-11-05), nada implementado.** Sai do item 2.4 de
> `docs/plano-erro-de-deploy-invisivel.md`. Aguarda as decisões da seção 4.

## 1. O problema, em linguagem simples

Hoje o log de um container só existe **dentro do container**. O daemon lê esse log ao vivo do Docker/Podman
(`LogsGet` e o fluxo de eventos `LogLine`); não guarda nada. Três situações em que isso dói:

1. **O deploy troca o container.** No estágio `Promoting` o container antigo é removido
   (`features/deploy/executor/promote.rs`) e o log dele some junto. "O que aconteceu antes do último deploy?"
   não tem resposta.
2. **O deploy falha.** O container novo (de *staging*) que não passou no healthcheck é removido no rollback, e o
   log dele, que é a evidência da falha, vai embora. O daemon acompanha só o container que já é o `live`
   (`features/observability/container_logs.rs` olha `live_container_id` a cada 5 s), então um container de staging
   que cai em segundos nunca é lido por ninguém.
3. **O log do Docker é do Docker.** O driver `json-file` roda e trunca por conta própria, sem o rustploy saber.

O que já existe e ajuda: o `stream_loop` **já lê continuamente** o log de cada serviço em execução e o publica no
barramento de eventos. Persistir é, em grande parte, **gravar também o que já passa por ali**.

## 2. O desenho proposto

**Gravar em arquivos, por serviço, e não em tabela do SQLite.** Serviço tagarela encheria o banco (e o backup dele);
log é texto sequencial que só se anexa no fim e se apaga do começo, e arquivo faz isso de graça.

- Pasta: `<data_dir>/logs/<service_id>/`, com *segmentos* `000001.jsonl`, `000002.jsonl`… Cada linha é um JSON:
  `{"t": "<RFC3339 UTC>", "c": "<container id curto>", "s": "out|err", "m": "<texto>"}`.
- Um segmento fecha ao chegar a ~4 MiB. A **retenção** apaga segmentos inteiros, do mais antigo para o mais novo,
  quando passam de `max_bytes_per_service` ou de `retention_days`. Nada de reescrever arquivo.
- Um único *escritor* (uma tarefa com canal) recebe as linhas e grava em lote (descarrega a cada ~1 s ou 200 linhas,
  e sempre que um stream acaba). O caminho quente do daemon não espera o disco.
- **Sem duplicar na reconexão.** Hoje o stream abre com `tail: 100`; ao reconectar, repetiria as últimas linhas. O
  escritor guarda o último `t` gravado por container e o stream reabre com `since` a partir dele.
- **Captura final antes de destruir.** Antes de remover um container (troca em `Promoting` e rollback de staging),
  ler o que ainda não foi gravado (`docker logs --since <último t>`) e gravar. Esta é a parte que resolve os itens
  1 e 2 do problema, inclusive para o container de staging que caiu em segundos.
- **Limpeza.** `ServiceDelete` apaga a pasta do serviço. Pastas sem serviço correspondente são apagadas no boot.
- **Compose.** Serviço Compose tem vários containers; o `stream_loop` hoje só segue um (`live_container_id`). Fica
  **fora desta primeira entrega** (continua só ao vivo), anotado como pendência.

### Como ler o histórico

Comando novo, aditivo: `LogsHistory { service_id, before?: timestamp, limit, container?: id }`, resposta
`LogsHistory(Vec<LogEntry>)` em ordem cronológica e `next_before` para paginar para trás (mesmo espírito do cursor do
`build_logs`). `LogsGet` continua igual (ao vivo). O manual do agente ganha a linha do comando.

**Telas (GUI e webui):** na janela de logs do serviço, um botão "Carregar anteriores" que chama o `LogsHistory` e
põe as linhas acima das ao vivo, com um separador "container anterior" quando o `c` muda. Os dois clientes, como
manda o `AGENTS.md`.

### Segurança

Log de aplicação pode conter segredo (a app imprime a própria senha). Hoje isso já é verdade ao vivo; persistir
**prolonga** a exposição. Duas medidas: (1) a pasta nasce com permissão `0700` e os arquivos `0600`; (2) o
`LogsHistory` passa pelo mesmo mascaramento de `api/agent/redigir.rs` quando a chamada vem do agente (MCP), como o
resto das respostas de texto. A retenção curta por padrão também reduz o risco.

## 3. Configuração

Seção nova no `config.toml` (mesmo idioma do `[deploy] image_cache`), toda opcional:

```toml
[logs]
enabled = true              # liga a gravação
retention_days = 14         # apaga segmentos mais velhos que isso
max_bytes_per_service = 52428800   # 50 MiB por serviço
```

Lembrete do AGENTS.md: o parse do config é **tudo-ou-nada**; a seção nova entra com `#[serde(default)]` e todos os
campos têm padrão, então um `config.toml` antigo continua carregando.

## 4. Decisões que preciso do dono

1. **Onde fica a retenção: só no daemon (recomendado), no cliente, ou nos dois?** Recomendo só no daemon: é a fonte
   única, serve GUI, webui e agente, e sobrevive a trocar de máquina. Cache local no cliente (ideia do plano antigo)
   fica de fora, sem ganho claro agora.
2. **Limites padrão:** 14 dias e 50 MiB por serviço servem? São por serviço; 8 serviços cheios ocupariam ~400 MiB no
   máximo.
3. **Ligado por padrão?** Recomendo que sim (a graça é ter o log quando se precisa). Quem não quiser põe
   `enabled = false`.
4. **Telas agora ou depois?** Recomendo fazer o backend + `LogsHistory` + agente primeiro e as telas na sequência,
   no mesmo publish se der tempo; sem tela o histórico só é alcançável por agente/API.

## 5. Ordem de implementação

1. `[logs]` no config + módulo `platform/log_store` (segmentos, retenção, escritor em lote) com testes
   (rotação, retenção por tamanho e por idade, recuperação de linha parcial no fim do arquivo após queda).
2. Ligar o `container_logs::stream_container` ao escritor, com `since` na reconexão.
3. Captura final no `Promoting` e no rollback de staging (e no `ServiceDelete`: apagar a pasta).
4. `LogsHistory` em `shared` (**protocolo**: sobe o `rustploy-shared`, junto com o 0.17.0 que já está pendente) e
   handler no daemon, com o mascaramento do agente.
5. Manual do agente, docs do site (guia de logs, referência regenerada).
6. Telas (GUI + webui) e testes headless/Luau.
7. Teste real: servidor local, serviço que imprime e cai, deploy que troca de container; conferir o histórico dos
   dois containers e o rollback de um deploy com healthcheck falhando.

## 6. O que fica de fora

- Logs de **Compose** com vários containers (só ao vivo, como hoje).
- Busca de texto, filtros por nível e exportação.
- Envio dos logs para fora (Loki etc.): há uma branch `main-loki` no chickie, mas é outro assunto.
