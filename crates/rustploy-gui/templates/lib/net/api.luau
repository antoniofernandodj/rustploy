-- net/api.luau — cliente HTTP/JSON da API do daemon (sucessor do RwpClient).
--
-- Encapsula base_url + header Authorization e expõe `rpc(cmd)`: faz
-- POST /api/rpc com o Command em JSON e devolve a Response decodificada. O
-- `fetch` do prelúdio SUSPENDE a corrotina da ação (async/await) — a UI não
-- trava enquanto espera a resposta.
--
-- Convenção de Command (serde externally-tagged):
--   • variante unitária  → string:  "DaemonStatus", "StopAllManaged"
--   • variante com campos → tabela:  { ProjectDelete = { id = "..." } }
-- Convenção de Response (idem): { Projects = {...} }, { DaemonStatus = {...} }.

local M = {}
M.__index = M

function M.new(base_url, token)
    return setmetatable({
        base_url = base_url,
        token = (token ~= nil and token ~= "") and token or nil,
    }, M)
end

function M:headers()
    local h = { ["Content-Type"] = "application/json" }
    if self.token then
        h["Authorization"] = "Bearer " .. self.token
    end
    return h
end

--- Executa um Command. Retorna (true, response_table) em sucesso HTTP, ou
--- (false, mensagem) em falha de transporte / corpo inválido.
function M:rpc(cmd)
    local res = fetch(self.base_url .. "/api/rpc", {
        method = "POST",
        body = json.encode(cmd),
        headers = self:headers(),
    })
    if not res.ok then
        local msg = (res.error ~= nil and res.error ~= "") and res.error
            or ("HTTP " .. tostring(res.status))
        return false, msg
    end
    local ok, decoded = pcall(json.decode, res.body)
    if not ok or type(decoded) ~= "table" then
        return false, "resposta inválida do daemon"
    end
    return true, decoded
end

--- Como `rpc`, mas trata `Response::Err { code, message }` como falha: retorna
--- (false, "code: message"). Em sucesso, (true, response_table).
function M:rpc_checked(cmd)
    local ok, res = self:rpc(cmd)
    if not ok then return false, res end
    if res.Err ~= nil then
        return false, tostring(res.Err.code) .. ": " .. tostring(res.Err.message)
    end
    return true, res
end

return M
