--!strict
-- helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem
-- estado, sem I/O). Cada handler faz `local Helpers = require("helpers")`.

local M = {}

function M.trim(s: any): string
    return (tostring(s or ""):gsub("^%s+", ""):gsub("%s+$", ""));
end

-- Toasts (built-in do glacier 0.20+): notificação efêmera, complementa o
-- `*_msg` inline. Usados nos desfechos de mutação (erro/sucesso).
function M.notify_err(msg: any): ()
    toast({ message = tostring(msg), kind = "error" });
end

function M.notify_ok(msg: any): ()
    toast({ message = tostring(msg), kind = "success" });
end

-- URI de callback OAuth do Gitea que o usuário registra no app do Gitea.
-- `base` é a URL pública do daemon (`DaemonSettings.public_base_url`), derivada
-- de [api] domain/port — o callback OAuth é servido pelo mesmo listener da API.
function M.oauth_redirect_uri(base: string?): string
    local d = M.trim(base):gsub("/+$", "");
    if d == "" then
        return "<URL pública do daemon indisponível>/oauth/gitea/callback";
    end
    return d .. "/oauth/gitea/callback";
end

-- Heurística: a URL denota um repo Git (clonar+buildar) e não uma imagem.
function M.looks_like_git_url(u: string?): boolean
    local s = M.trim(u);
    return s:match("^https://") ~= nil or s:match("^http://") ~= nil
        or s:match("^git@") ~= nil or s:match("^ssh://") ~= nil
        or s:match("^file://") ~= nil or s:match("%.git$") ~= nil
end

-- Normaliza a entrada de URL do login para uma base HTTP(S) sem barra final.
-- Aceita "https://host:porta", "http://host:porta", "host:porta", "host".
-- Sem esquema → https. Outros esquemas (ex.: ws, ftp) são rejeitados.
function M.normalize_url(raw: string): string?
    local u = (raw or ""):gsub("^%s+", ""):gsub("%s+$", "");
    if u == "" then
        return nil;
    end
    local scheme, rest = string.match(u, "^([%a][%w+.-]*)://(.*)$");
    if scheme then
        scheme = string.lower(scheme);
        if scheme ~= "http" and scheme ~= "https" then
            return nil;
        end
    else
        scheme, rest = "https", u
    end
    -- corta path/query/fragment: só queremos o authority (host[:porta]).
    rest = string.match(rest or "", "^([^/?#]*)") or rest;
    if rest == "" then
        return nil;
    end
    return (scheme or "") .. "://" .. (rest or "");
end

return M
