// net/sse.js — consumidor de endpoints SSE do daemon: o firehose
// `/api/events` (porta de crates/rustploy-gui/views/scripts/handlers/
// stream.luau — só a parte de transporte, a aplicação do snapshot/bus mora
// em app.js/screens/*.js) e os endpoints dedicados de logs por serviço/
// deployment (`/api/services/{id}/logs`, `/api/deployments/{id}/build-logs`
// — ver crates/daemon/src/api/http_api.rs), que usam o MESMO framing SSE.
//
// Não usa `EventSource` nativo: ele não permite mandar o header
// `Authorization`, e toda rota SSE do daemon exige o mesmo Bearer token de
// `/api/rpc`. Em vez disso, lê o corpo da resposta como stream de bytes
// (mesmo protocolo text/event-stream, decodificado à mão) via `fetch` +
// `ReadableStream`.

/**
 * Abre a stream em `path` (relativo a `baseUrl`, ex. "/api/events" ou
 * "/api/services/<id>/logs") e devolve um controlador com `close()`.
 * @param {string} baseUrl
 * @param {string} token
 * @param {string} path
 * @param {{onEvent:(kind:string,data:any)=>void, onError?:(msg:string)=>void, onClose?:()=>void}} handlers
 */
export function openStream(baseUrl, token, path, handlers) {
  const controller = new AbortController();
  let closedByUs = false;

  (async () => {
    let res;
    try {
      res = await fetch(baseUrl.replace(/\/+$/, "") + path, {
        headers: token ? { Authorization: "Bearer " + token } : {},
        signal: controller.signal,
      });
    } catch (e) {
      if (!closedByUs) handlers.onError?.(e && e.message ? e.message : "falha ao conectar");
      handlers.onClose?.();
      return;
    }
    if (!res.ok || !res.body) {
      handlers.onError?.("HTTP " + res.status);
      handlers.onClose?.();
      return;
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder("utf-8");
    let buf = "";
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });

        let sep;
        while ((sep = buf.indexOf("\n\n")) !== -1) {
          const record = buf.slice(0, sep);
          buf = buf.slice(sep + 2);
          let kind = "message";
          let data = null;
          for (const line of record.split("\n")) {
            if (line.startsWith("event:")) kind = line.slice(6).trim();
            else if (line.startsWith("data:")) data = line.slice(5).trim();
          }
          if (data !== null) {
            try {
              handlers.onEvent(kind, JSON.parse(data));
            } catch {
              // linha malformada — ignora, próximo frame segue normal
            }
          }
        }
      }
    } catch (e) {
      if (!closedByUs) handlers.onError?.(e && e.message ? e.message : "stream interrompida");
    }
    if (!closedByUs) handlers.onClose?.();
  })();

  return {
    close() {
      closedByUs = true;
      controller.abort();
    },
  };
}
