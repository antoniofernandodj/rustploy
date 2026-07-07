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

declare function fetch(url: string, opts: any?): FetchResult
declare function sse(url: string, opts: any?): StreamHandle
declare function websocket(url: string, opts: any?): StreamHandle
declare function toast(opts: any): ()
declare function confirm(opts: any): ()

declare json: {
    decode: (s: string) -> any,
    encode: (v: any) -> string,
    array: <T>(t: { T }) -> { T },
}
