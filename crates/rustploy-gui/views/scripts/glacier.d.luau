--!strict
-- Definições dos globais que o motor glacier-ui injeta no interpretador Luau em
-- runtime (não existem como arquivo). Consumido só pelo type-checker (luau-lsp):
-- o mlua ignora tipos/anotações em runtime. Carregue com
--   luau-lsp analyze --definitions=views/scripts/glacier.d.luau ...

export type FetchResult = {
    ok: boolean,
    status: number,
    body: string,
    error: string,
}

export type StreamHandle = {
    id: number,
    close: (self: StreamHandle) -> any,
    send: (self: StreamHandle, text: string) -> any,
}

-- Contexto global do motor: chave→valor string (o motor coage tudo p/ string).
declare ctx: { [string]: string }

-- Valor do on_change/on_toggle/ação-com-sufixo (nil quando a ação não traz valor).
declare value: string?

-- Tamanho atual da janela em px lógicos (o que também alimenta @media no GSS).
export type Viewport = {
    width: number,
    height: number,
}
declare function viewport(): Viewport

-- HTTP, ou leitura de arquivo local quando a URL começa com `file://`
-- (glacier-ui 0.49+): devolve o mesmo FetchResult, com o conteúdo no `body`.
declare function fetch(url: string, opts: any?): FetchResult
-- Escrita de arquivo local (glacier-ui 0.49+), contraparte do `fetch("file://…")`.
-- Devolve `true` no sucesso, ou `false, mensagem` na falha.
declare function write_file(path: string, contents: string): (boolean, string?)
-- Streams de vida longa. NÃO suspendem: registram o stream e devolvem o handle
-- na hora. Cada callback é uma FUNÇÃO (closure e upvalue funcionam) — o motor
-- também aceita o nome de uma função global, como atalho, mas o tipo abaixo
-- guia para a forma boa: o nome obriga o handler a ser global, não fecha sobre
-- nada, e falha em silêncio quando está errado.
export type StreamOptions = {
    headers: { [string]: string }?,
    on_open: (() -> ())?,
    on_message: ((data: string) -> ())?,
    on_error: ((message: string) -> ())?,
    on_close: (() -> ())?,
}
declare function sse(url: string, opts: StreamOptions?): StreamHandle
declare function websocket(url: string, opts: StreamOptions?): StreamHandle
declare function toast(opts: any): ()
-- Notificação nativa do SO (glacier-ui 0.42+): `notify({ title?, body? })` ou
-- `notify("mensagem")`. Diferente de `toast` (efêmero, dentro da janela).
declare function notify(opts: any): ()
declare function confirm(opts: any): boolean

-- Multi-janela (glacier-ui 0.36+/0.37+): abrir janela, mensagear entre janelas e
-- fechar a própria. `on_broadcast(event, payload)` é definido pelo usuário (a
-- janela receptora), não injetado — por isso não é declarado aqui.
declare function open_window(opts: any): ()
declare function broadcast(event: string, payload: any?): ()
declare function close_window(): ()

-- Anexa texto ao fim do <textarea> ligado a `binding` (chave do `value`), de
-- forma incremental (o motor insere no Content sem recriá-lo — preserva scroll).
declare function append_textarea(binding: string, text: string): ()

declare json: {
    decode: (s: string) -> any,
    encode: (v: any) -> string,
    array: <T>(t: { T }) -> { T },
}

-- Persistência local em JSON, análoga ao `localStorage` do browser: sobrevive a
-- um restart do processo (ao contrário do `ctx`, que é só memória). `set` grava
-- qualquer valor JSON-serializável (string, número, booleano, tabela); `get`
-- devolve o valor guardado ou nil; `remove` apaga a chave. No rustploy grava em
-- `~/.local/share/rustploy/.glacier-storage/app.json` (raiz definida no Rust via
-- `GlacierDaemon::storage_dir`, glacier-ui 0.45+).
declare storage: {
    get: (key: string) -> any,
    set: (key: string, value: any) -> (),
    remove: (key: string) -> (),
}

-- Data e hora sobre strings ISO (glacier-ui 0.72+; fuso e epoch na 0.73). Um
-- valor SEM fuso é hora local — é o que `today`/`now` devolvem e o que o
-- `<timeedit>` guarda; um valor COM fuso (`2026-07-06T12:34:56Z`, `...-03:00`,
-- fração de segundo opcional) usa o dele. É esse segundo formato que o daemon
-- manda em todo `started_at`/`created_at`, e `to_local` é o que o traz para a
-- hora de parede da tela. Ver `fmt/time.luau`.
export type DateDelta = {
    years: number?,
    months: number?,
    days: number?,
    hours: number?,
    minutes: number?,
    seconds: number?,
}

declare date: {
    today: () -> string,
    now: (segundos: boolean?) -> string,
    time: (segundos: boolean?) -> string,
    parse: (iso: string) -> { [string]: number }?,
    valid: (iso: string) -> boolean,
    weekday: (iso: string) -> number?,
    date_of: (iso: string) -> string?,
    time_of: (iso: string) -> string?,
    days_in_month: (ano: number, mes: number) -> number,
    compare: (a: string, b: string) -> number?,
    is_before: (a: string, b: string) -> boolean,
    is_after: (a: string, b: string) -> boolean,
    add: (iso: string, delta: DateDelta?) -> string?,
    diff: (a: string, b: string) -> number?,
    diff_seconds: (a: string, b: string) -> number?,
    format: (iso: string, fmt: string) -> string?,
    epoch: (iso: string) -> number?,
    from_epoch: (segundos: number, utc: boolean?) -> string?,
    to_local: (iso: string) -> string?,
    to_utc: (iso: string) -> string?,
}
