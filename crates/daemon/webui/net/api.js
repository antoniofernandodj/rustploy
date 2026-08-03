// net/api.js — cliente HTTP/JSON do daemon. Porta de
// crates/rustploy-gui/views/scripts/net/api.luau: mesma convenção de
// Command/Response (serde externally-tagged) e mesma autenticação Bearer.
//
//   • variante unitária  → string:  "DaemonStatus", "StopAllManaged"
//   • variante com campos → objeto: { ProjectDelete: { id: "..." } }

export class Api {
  constructor(baseUrl, token) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.token = token || "";
  }

  headers() {
    const h = { "Content-Type": "application/json" };
    if (this.token) h["Authorization"] = "Bearer " + this.token;
    return h;
  }

  /** Executa um Command. Retorna { ok: true, value } ou { ok: false, error }. */
  async rpc(cmd) {
    let res;
    try {
      res = await fetch(this.baseUrl + "/api/rpc", {
        method: "POST",
        headers: this.headers(),
        body: JSON.stringify(cmd),
      });
    } catch (e) {
      return { ok: false, error: e && e.message ? e.message : "falha de rede" };
    }
    if (!res.ok) {
      return { ok: false, error: "HTTP " + res.status };
    }
    let decoded;
    try {
      decoded = await res.json();
    } catch {
      return { ok: false, error: "resposta inválida do daemon" };
    }
    return { ok: true, value: decoded };
  }

  /** Como rpc(), mas trata `Response::Err { code, message }` como falha. */
  async rpcChecked(cmd) {
    const r = await this.rpc(cmd);
    if (!r.ok) return r;
    const v = r.value;
    if (v && typeof v === "object" && v.Err) {
      return { ok: false, error: v.Err.message || v.Err.code || "erro" };
    }
    return r;
  }
}
