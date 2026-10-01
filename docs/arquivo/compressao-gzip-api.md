# Compressão gzip da API (daemon → GUI)

**Status: FEITO** (glacier-ui 0.51.0 + daemon).

## O problema

A GUI conversa com o daemon por HTTP/JSON + SSE. Quando a GUI roda na mesma
máquina que o daemon (`127.0.0.1`), o tamanho das respostas quase não importa —
o loopback é rápido. Mas o rustploy é um PaaS de VPS: o caso de uso real é a
**GUI num laptop conectada a um daemon remoto**. Aí cada resposta JSON grande (o
catálogo de templates, o snapshot de projetos/serviços) trafega inteira pela
rede, e JSON é muito redundante — comprime 70–90%.

## A solução

Negociação de conteúdo padrão do HTTP (`Accept-Encoding`/`Content-Encoding`),
**só para as respostas JSON unárias** (`POST /api/rpc`). O SSE ficou de fora de
propósito (ver abaixo).

### Cliente (glacier-ui `src/net.rs`, 0.51.0)

- O `fetch` manda `Accept-Encoding: gzip` por padrão (a menos que o chamador já
  tenha definido um `Accept-Encoding`).
- Se a resposta vier com `Content-Encoding: gzip`, o corpo é descomprimido antes
  de chegar ao Lua — que recebe o **mesmo** `body` de texto de sempre. Falha de
  descompressão vira `ok=false`/`error`, nunca lixo.
- É genérico: qualquer app glacier ganha isso, não só o rustploy.

### Servidor (daemon `crates/daemon/src/api/http_api.rs`)

- `accepts_gzip(req)` lê o `Accept-Encoding` da request **antes** de consumir o
  corpo.
- `json_response(bytes, accept_gzip)` comprime com gzip (`flate2`, o mesmo crate
  do tar do build Docker) **quando** o cliente aceita **e** o corpo passa de
  `GZIP_MIN` (1 KB) — abaixo disso o overhead do cabeçalho + CPU não compensa.
  Seta `Content-Encoding: gzip`. Falha ao comprimir (rara) → manda cru.

## Por que o SSE ficou de fora

O SSE (`/api/events`, logs e métricas ao vivo) é um stream de vida longa. Não dá
para gzipar o corpo inteiro como um blob — teria que ser um gzip com **flush por
evento**, o que piora a taxa de compressão, adiciona complexidade no parser do
cliente e costuma dar atrito com proxies (que já é o motivo do
`X-Accel-Buffering: no`). Cada evento é pequeno (uma linha de log), então o ganho
seria pequeno. Fica para uma segunda fase, se algum dia valer a pena.

## Testes

- glacier: `gunzip_round_trip` (`src/net.rs`).
- daemon: `json_response_comprime_grande_e_deixa_pequeno_cru`
  (`crates/daemon/src/api/http_api.rs`) — comprime acima do limiar com o cliente
  aceitando, deixa cru abaixo do limiar ou sem aceite, e o gzip é reversível.

O ponto de integração é `Content-Encoding` puro do HTTP, então os dois testes
unitários (um de cada lado) cobrem o contrato.
