//! `GET /agent/schema` — o documento de descoberta.
//!
//! Sem isto, montar a primeira chamada exige ler `crates/shared/src/protocol.rs`
//! e `models.rs` e deduzir a codificação serde na mão — viável para quem tem o
//! repositório aberto, inviável para um agente diante de um daemon remoto. É o
//! atrito mais caro relatado em `docs/plano-erro-de-deploy-invisivel.md` (2.1).
//!
//! **Este catálogo é curado, não gerado.** As rotas desta API estão descritas
//! por inteiro; a lista de `Command` cobre o que aparece em runbook, não as ~90
//! variantes do enum. A fonte da verdade continua sendo `protocol.rs`, e o
//! passthrough (`POST /agent/rpc`) aceita qualquer comando, esteja ele listado
//! aqui ou não — inclusive os que forem adicionados depois deste arquivo.

use serde_json::{json, Value};

pub(super) fn schema() -> Value {
    json!({
        "service": "rustploy-gui agent API",
        "version": 1,
        "summary":
            "Ponte local para o daemon rustploy REMOTO ao qual esta janela está \
             conectada. As credenciais do daemon ficam no app; você usa o token \
             do arquivo de handoff. Trocar de servidor na GUI troca o alvo.",
        "auth": {
            "scheme": "Authorization: Bearer <token>",
            "token_source":
                "arquivo de handoff agent-api.json no data dir do usuário \
                 (~/.local/share/rustploy no Linux); campos url/token/pid",
            "exempt": ["GET /agent/health"]
        },
        "conventions": {
            "errors":
                "toda falha é JSON: {\"error\":{\"code\":\"…\",\"message\":\"…\"}}",
            "status_codes": {
                "400": "pedido malformado, ou o daemon recusou o comando (Response::Err)",
                "401": "token desta API ausente/errado",
                "404": "rota ou serviço inexistente",
                "502": "o daemon remoto não respondeu, ou respondeu algo inesperado",
                "503": "a janela do Rustploy não está conectada a daemon nenhum"
            }
        },
        "routes": [
            {
                "method": "GET", "path": "/agent/health",
                "auth": false,
                "does": "liveness da ponte + se a janela está conectada",
                "returns": "{ok, app, connected}"
            },
            {
                "method": "GET", "path": "/agent/schema",
                "does": "este documento"
            },
            {
                "method": "GET", "path": "/agent/status",
                "does": "versão/uptime do daemon remoto + estado da fila de deploys",
                "returns": "{connected, remote_url, daemon, deploy_engine}"
            },
            {
                "method": "GET", "path": "/agent/services",
                "does":
                    "índice achatado projeto→serviço: id, nome, status e origem. \
                     Use para achar o service_id a partir do nome.",
                "returns": "{count, services:[{service_id, name, project, project_id, status, source, port}]}"
            },
            {
                "method": "GET", "path": "/agent/deploys",
                "query": { "limit": "1..200, padrão 20" },
                "does":
                    "últimos deploys de todos os serviços, com o desfecho já \
                     resolvido — `ok` e `error` sem precisar garimpar states_log",
                "returns":
                    "{count, deployments:[{deployment_id, service, project, state, \
                     ok, error, started_at, finished_at}]}",
                "notes":
                    "ok: true=Live, false=Failed, null=em andamento ou terminal \
                     sem desfecho (Stopped, Pruning)"
            },
            {
                "method": "POST", "path": "/agent/deploys",
                "body": {
                    "service_id": "id do serviço (ou use \"service\")",
                    "service": "nome do serviço; erro se ambíguo entre projetos",
                    "wait": "bool, padrão true — só responde quando o deploy terminar",
                    "timeout_s": "espera máxima, padrão 900, teto 3600",
                    "log_tail": "linhas finais do build log a devolver, padrão 40, teto 500"
                },
                "does": "dispara o deploy e devolve como ele terminou",
                "returns":
                    "{deployment_id, state, ok, error, log_tail, log_cursor, \
                     waited, timed_out}  (202 e sem desfecho quando wait=false)",
                "notes":
                    "a rota principal: com wait=true, uma chamada responde \
                     'funcionou?' e 'por quê não?' de uma vez"
            },
            {
                "method": "GET", "path": "/agent/deploys/<deployment_id>/logs",
                "query": {
                    "after": "índice da última linha já vista, padrão 0",
                    "limit": "1..5000, padrão 500"
                },
                "does": "build log paginado por cursor",
                "returns": "{total, after, next_after, has_more, lines:[…]}",
                "notes":
                    "passe o next_after da resposta anterior para pegar só o que \
                     surgiu desde então — o daemon só sabe devolver o log inteiro"
            },
            {
                "method": "GET", "path": "/agent/ingress",
                "does": "tabela de rotas VIVA do ingress proxy (domínios + portas)",
                "returns": "{domains:[{domain, backends:[ip:porta], service_id}], ports:[{host_port, backends}]}",
                "notes":
                    "é o diagnóstico de um domínio que responde 502: a rota \
                     existe? aponta para qual ip:porta? backends vazio = rota \
                     registrada sem destino"
            },
            {
                "method": "POST", "path": "/agent/ingress/reconcile",
                "body": { "service_id": "opcional; omitido = todos os serviços" },
                "does": "recalcula as rotas a partir dos containers reais, sem redeploy",
                "returns": "a tabela já corrigida, no formato de GET /agent/ingress"
            },
            {
                "method": "POST", "path": "/agent/rpc",
                "body": "um Command do protocolo, em JSON",
                "does": "passthrough cru para POST /api/rpc do daemon",
                "returns": "a Response do daemon, verbatim (Response::Err vira 400)"
            },
            {
                "method": "POST", "path": "/agent/services/<service_id>/archive",
                "body": { "path": "caminho LOCAL do .zip (a ponte é loopback)" },
                "does": "sobe o zip de um serviço com fonte Archive",
                "notes":
                    "no daemon isto é rota HTTP com corpo binário, NÃO um \
                     Command — não dá para descobrir lendo protocol.rs"
            },

            {
                "method": "GET", "path": "/agent/servers",
                "does": "servidores que o usuário já usou nesta máquina",
                "returns": "{count, servers:[{url, has_saved_token}]}",
                "notes": "o token salvo nunca sai — só a informação de que existe"
            },
            {
                "method": "POST", "path": "/agent/connect",
                "body": {
                    "url": "base do daemon, ex. https://rustploy.exemplo.com",
                    "token": "opcional — omitido, usa o salvo para essa URL"
                },
                "does":
                    "entra na sessão pela própria tela de login da GUI e espera \
                     o desfecho (a janela acompanha: abre o SSE e vai para o shell)",
                "returns": "{connected:true, remote_url} — 502 se o daemon recusou",
                "notes":
                    "é o que dispensa um humano na frente do app; sem isto toda \
                     rota com dado responde 503 not_connected"
            },
            {
                "method": "POST", "path": "/agent/disconnect",
                "does": "sai da sessão (fecha o SSE, limpa o contexto, volta ao login)"
            },
            {
                "method": "GET", "path": "/agent/ui",
                "query": {
                    "keys": "lista de chaves do contexto, separadas por vírgula",
                    "all": "1 para o contexto inteiro (resposta grande)"
                },
                "does": "o que a janela está mostrando agora",
                "returns":
                    "{connected, remote_url, screen, view, selected_project, \
                     selected_service, search, status_line, error, data_loading, counts}",
                "notes": "api_token e token saem sempre como \"<redigido>\""
            },
            {
                "method": "GET", "path": "/agent/ui/actions",
                "does": "lista TODAS as ações que a GUI aceita, com o arquivo de origem",
                "returns": "{count, actions:[{action, source}], how_to_call}",
                "notes":
                    "lido da árvore de scripts em execução, não de uma lista \
                     escrita à mão — é o chaveiro do POST /agent/ui/action"
            },
            {
                "method": "POST", "path": "/agent/ui/action",
                "body": {
                    "action": "nome da ação (função Luau global)",
                    "value": "opcional — com valor é onChange, sem valor é clique"
                },
                "does": "dispara QUALQUER ação da GUI, como se fosse um clique",
                "returns": "202 {dispatched} — o efeito é assíncrono",
                "notes":
                    "a chave-mestra: todo botão/aba/formulário da GUI é uma \
                     função global em views/scripts/handlers/*.luau, e todas são \
                     alcançáveis por aqui. Ver o manual em AGENTS.md."
            },
            {
                "method": "POST", "path": "/agent/ui/context",
                "body": "objeto de pares chave→valor a escrever no contexto",
                "does": "preenche campo de formulário, marca seleção, troca aba",
                "returns": "202 {patched:[chaves]}",
                "notes":
                    "o par de baixo nível do ui/action — use quando a ação que \
                     você quer disparar espera algo já escrito no contexto"
            }
        ],
        "protocol": {
            "encoding":
                "serde externally-tagged. Variante com campos é objeto de uma \
                 chave — {\"ProjectCreate\":{\"name\":\"x\",\"description\":null}}; \
                 variante sem campos é a string nua — \"ProjectList\". Vale para \
                 Command e para Response.",
            "source_of_truth":
                "crates/shared/src/protocol.rs (Command/Response) e models.rs \
                 (ServiceSpec, ServiceSource, EnvVar, Healthcheck…)",
            "not_a_command": {
                "archive_upload":
                    "subir um zip é rota HTTP separada no daemon \
                     (POST /api/services/<id>/archive, corpo binário), não um \
                     Command — não dá para descobrir isso lendo protocol.rs"
            },
            "common_commands": [
                { "cmd": "\"ProjectList\"", "does": "lista projetos" },
                { "cmd": "{\"ProjectCreate\":{\"name\":\"loja\",\"description\":null}}",
                  "does": "cria projeto" },
                { "cmd": "{\"ServiceList\":{\"project_id\":\"proj_…\"}}",
                  "does": "serviços de um projeto" },
                { "cmd": "{\"ServiceGet\":{\"id\":\"svc_…\"}}",
                  "does": "o ServiceSpec completo de um serviço" },
                { "cmd": "{\"ServiceUpdate\":{\"id\":\"svc_…\",\"spec\":\"<o ServiceSpec inteiro, vindo de ServiceGet>\"}}",
                  "does": "substitui o spec",
                  "careful":
                      "é SUBSTITUIÇÃO TOTAL, não patch: faça ServiceGet, mude o \
                       campo e devolva o spec inteiro. Campo omitido é campo apagado." },
                { "cmd": "{\"DeployStart\":{\"service_id\":\"svc_…\"}}",
                  "does": "dispara deploy",
                  "careful": "prefira POST /agent/deploys, que já espera o desfecho" },
                { "cmd": "{\"DeployHistory\":{\"service_id\":\"svc_…\",\"limit\":5}}",
                  "does": "deployments do serviço, com states_log (onde mora a causa da falha)" },
                { "cmd": "{\"GetBuildLogs\":{\"deployment_id\":\"dep_…\"}}",
                  "does": "build log inteiro",
                  "careful": "sem cursor; prefira GET /agent/deploys/<id>/logs?after=" },
                { "cmd": "{\"LogsGet\":{\"service_id\":\"svc_…\",\"tail\":200}}",
                  "does": "log de runtime do container",
                  "careful":
                      "lido do Docker ao vivo, não persistido: o swap de deploy \
                       destrói o container antigo e o log dele some junto" },
                { "cmd": "{\"ServiceStop\":{\"service_id\":\"svc_…\"}}", "does": "para o serviço" },
                { "cmd": "{\"DeployRollback\":{\"service_id\":\"svc_…\"}}",
                  "does": "volta para o deployment anterior" },
                { "cmd": "{\"SecretSet\":{\"project_id\":\"proj_…\",\"name\":\"X\",\"value\":\"…\"}}",
                  "does": "grava um secret cifrado do projeto" },
                { "cmd": "\"DaemonStatus\"", "does": "versão/uptime/serviços no ar" },
                { "cmd": "\"DeployEngineStatus\"", "does": "fila de deploys: ativos, enfileirados, recentes" },
                { "cmd": "\"ManifestExportAll\"",
                  "does": "exporta todos os projetos como manifesto YAML + .env" },
                { "cmd": "{\"ManifestImport\":{\"yaml\":\"…\",\"dotenv\":\"…\",\"prune\":false,\"deploy\":false}}",
                  "does": "aplica manifesto (infra-as-code)" }
            ]
        },
        "gotchas": [
            "Um deploy que falha vira ok=false com a causa em `error` — texto do \
             docker build, healthcheck que não passou, pull negado. Antes da \
             correção do log de falha essa causa só existia no SQLite do daemon.",
            "O nome do serviço é único por projeto, não globalmente: POST \
             /agent/deploys recusa um nome ambíguo em vez de escolher sozinho.",
            "Esta ponte tem o mesmo alcance da janela — que é o bearer do daemon, \
             hoje sem escopo. Não há modo somente-leitura.",
            "Ações de UI (ui/action, ui/context, connect) são ENTREGUES, não \
             confirmadas: respondem 202 e o efeito é assíncrono. Confirme o \
             resultado em GET /agent/ui ou na rota de dado correspondente.",
            "A janela pode estar recolhida na bandeja: o motor segue vivo e \
             continua aceitando ações, então não é preciso abri-la para operar."
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O catálogo é escrito à mão; um `json!` malformado só apareceria em
    /// runtime. Este teste garante que ele ao menos existe e cita cada rota que
    /// o roteador realmente serve.
    #[test]
    fn catalogo_descreve_todas_as_rotas() {
        let s = schema();
        let rotas = s.get("routes").and_then(Value::as_array).unwrap();

        let caminhos: Vec<(&str, &str)> = rotas
            .iter()
            .map(|r| {
                (
                    r.get("method").and_then(Value::as_str).unwrap(),
                    r.get("path").and_then(Value::as_str).unwrap(),
                )
            })
            .collect();

        for esperada in [
            ("GET", "/agent/health"),
            ("GET", "/agent/schema"),
            ("GET", "/agent/status"),
            ("GET", "/agent/services"),
            ("GET", "/agent/deploys"),
            ("POST", "/agent/deploys"),
            ("GET", "/agent/deploys/<deployment_id>/logs"),
            ("GET", "/agent/ingress"),
            ("POST", "/agent/ingress/reconcile"),
            ("POST", "/agent/rpc"),
            ("POST", "/agent/services/<service_id>/archive"),
            ("GET", "/agent/servers"),
            ("POST", "/agent/connect"),
            ("POST", "/agent/disconnect"),
            ("GET", "/agent/ui"),
            ("GET", "/agent/ui/actions"),
            ("POST", "/agent/ui/action"),
            ("POST", "/agent/ui/context"),
        ] {
            assert!(caminhos.contains(&esperada), "faltou {esperada:?}");
        }
    }

    /// Cada exemplo de comando tem que ser JSON de verdade — copiar um exemplo
    /// quebrado do catálogo é pior do que não ter exemplo.
    #[test]
    fn exemplos_de_comando_sao_json_valido() {
        let s = schema();
        let cmds = s
            .pointer("/protocol/common_commands")
            .and_then(Value::as_array)
            .unwrap();
        assert!(!cmds.is_empty());

        for c in cmds {
            let texto = c.get("cmd").and_then(Value::as_str).unwrap();
            serde_json::from_str::<Value>(texto)
                .unwrap_or_else(|e| panic!("exemplo inválido {texto:?}: {e}"));
        }
    }
}
