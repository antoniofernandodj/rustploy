--!strict
-- Fixture do teste `fmt_service_detail.rs`: exercita `compose_host` e
-- `internal_url` de `fmt/service_detail.luau` através do motor e deixa cada
-- resultado numa chave de contexto. Mora fora da árvore de scripts do app pelo
-- mesmo motivo da `tempo.luau`: o `require` resolve por `GLACIER_LUAU_PATH`.
local SD = require("fmt/service_detail")

function init(): ()
    local yaml = "services:\n  # comentário\n\n  rp_banco:\n    image: postgres\n  outro:\n    image: y\n"

    -- A primeira chave de `services:`, pulando comentário e linha em branco.
    ctx.host_primeira = SD.compose_host(yaml, nil) or "nil"
    -- `ingress_service` declarado ganha da primeira chave.
    ctx.host_ingress = SD.compose_host(yaml, "kong") or "nil"
    -- Sem `services:`, ou com o bloco encerrado antes de uma chave: nil.
    ctx.host_sem_services = SD.compose_host("image: x", nil) or "nil"
    ctx.host_bloco_vazio = SD.compose_host("services:\nfoo: 1", nil) or "nil"
    ctx.host_nil = SD.compose_host(nil, nil) or "nil"

    -- Serviço Compose: o host é a chave do YAML, não `rp_<nome>`.
    ctx.url_compose = SD.internal_url("postgres", "meu_banco", 5432, SD.compose_host(yaml, nil))
    -- Application: `rp_<nome>`, como sempre foi.
    ctx.url_app = SD.internal_url(nil, "api", 80, nil)
    ctx.url_app_db = SD.internal_url("redis", "cache", 6379, nil)
end
