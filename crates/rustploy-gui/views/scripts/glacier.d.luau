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

declare function fetch(url: string, opts: any?): FetchResult
declare function sse(url: string, opts: any?): StreamHandle
declare function websocket(url: string, opts: any?): StreamHandle
declare function toast(opts: any): ()
declare function confirm(opts: any): ()

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
