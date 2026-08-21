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

  /** `POST /api/services/<id>/archive` — corpo binário cru (não é RPC JSON).
   * Porta de net/api.luau::upload_archive; aqui o `File` já traz os bytes
   * (sem o round-trip por base64 que o Luau precisa pro `fetch("file://…")`). */
  async uploadArchive(serviceId, file) {
    let res;
    try {
      const h = { "Content-Type": "application/zip" };
      if (this.token) h["Authorization"] = "Bearer " + this.token;
      h["X-Rustploy-Filename"] = file.name || "archive.zip";
      res = await fetch(`${this.baseUrl}/api/services/${serviceId}/archive`, {
        method: "POST",
        headers: h,
        body: file,
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
    if (decoded && typeof decoded === "object" && decoded.Err) {
      return { ok: false, error: decoded.Err.message || decoded.Err.code || "erro" };
    }
    return { ok: true, value: decoded };
  }
}
