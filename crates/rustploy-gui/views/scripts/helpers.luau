--!strict
-- helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem
-- estado, sem I/O). Cada handler faz `local Helpers = require("helpers")`.

local M = {}

function M.trim(s: any): string
    return (tostring(s or ""):gsub("^%s+", ""):gsub("%s+$", ""));
end

-- Referência a secret num valor de env var → nome do secret (ou nil se o valor
-- for texto comum). Aceita as DUAS formas em que a referência aparece no
-- projeto, porque cada uma é a sintaxe natural de um lugar:
--   <secret:NOME>  — editor .env/campo valor da GUI (o que o Exportar gera)
--   secret:NOME    — manifesto YAML do IaC (SECRET_PREFIX em shared/manifest.rs)
-- Aceitar só a primeira fazia `secret:NOME` (copiado de um manifesto ou da doc
-- antiga) virar valor LITERAL calado, e o container subia com a string
-- "secret:NOME" no lugar do segredo. Um único ponto de verdade para os quatro
-- call sites (env do serviço e do projeto, editor .env e campo valor) — foi a
-- duplicação entre eles que deixou as sintaxes divergirem.
function M.parse_secret_ref(v: string?): string?
    local s = M.trim(v)
    local name = s:match("^<secret:(.+)>$") or s:match("^secret:(.+)$")
    if name == nil then
        return nil
    end
    name = M.trim(name)
    return name ~= "" and name or nil
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
