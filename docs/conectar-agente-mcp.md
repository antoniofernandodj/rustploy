# Conectar um agente (Claude Code) ao MCP de um rustploy

> **Status: em uso** (2026-10-14). Guia permanente, não é plano. O desenho do
> token de conexão e do `/mcp` está em `docs/plano-chave-de-acesso-e-mcp.md`, e
> o manual que o daemon serve aos agentes está em
> `rustploy-daemon/crates/daemon/src/api/agent/manual.md`.

O daemon do rustploy serve um servidor MCP em `POST /mcp`. Com ele, um Claude
Code em **qualquer máquina** opera o servidor: lista serviços, dispara deploy
e espera o resultado, lê build log, diagnostica 502. O app desktop não precisa
estar aberto.

## O que você precisa: o token de conexão

É o base64 padrão (com `=`) de `host:porta::token_da_api`. Por exemplo,
`base64("rustploy.exemplo.com:9797::a3f9…")`. Ele não carrega o esquema
(`https`/`http`).

Onde pegar:

- no `config.toml` do daemon (`/etc/rustploy/config.toml`), em
  `[api].connection_token`. O daemon regrava esse valor a cada boot, logo
  abaixo do `token`;
- na VPS: `rustployd access-key`;
- numa sessão aberta da GUI ou da webui: Settings → Web Server → Token de
  conexão → Copiar.

> **Ele vale tanto quanto o token de administrador.** Não existe escopo nem
> modo somente-leitura, e comandos destrutivos não pedem confirmação. Cole o
> token só em máquinas suas. Se ele vazar, troque `[api].token` no
> `config.toml` e reinicie o daemon. O `connection_token` se regrava sozinho.

## Caminho curto: o comando pronto

Se você tem uma sessão aberta na GUI ou na webui, vá em Settings → Web Server
→ "Conectar um agente (Claude Code)" → Copiar. O comando já vem com a URL e o
token resolvidos:

```bash
claude mcp add --transport http --scope user rustploy https://HOST:PORTA/mcp \
  --header "Authorization: Bearer <token da API>"
```

Depois rode `claude mcp list` (o `rustploy` deve aparecer como
`✔ Connected`) e abra uma sessão **nova**, porque as ferramentas não aparecem
na sessão em que o comando rodou. Para conferir, use `/mcp`.

## Caminho com validação: o prompt para o agente

Use este caminho quando você só tem o token de conexão em mãos. Cole o prompt
abaixo no Claude Code da outra máquina, trocando `<TOKEN_DE_CONEXAO>`. O
agente decodifica o token, valida no servidor, registra o MCP e faz um teste
só de leitura.

```text
Quero conectar este Claude Code ao MCP do meu servidor Rustploy (um PaaS).

Token de conexão: <TOKEN_DE_CONEXAO>

É o base64 de `host:porta::token_da_api`. Faça nesta ordem:

1. Decodifique o token de conexão (`echo '<token>' | base64 -d`). Separe o
   `host:porta` e o token da API pelo primeiro `::`. Para conferir, me mostre
   só o `host:porta`; não repita o token da API na conversa.

2. Valide o acesso com o login do servidor. Tente https primeiro e use http
   só se o https falhar na conexão (erro de TLS ou sem resposta):
   curl -s https://HOST:PORTA/api/login -d '{"key":"<token de conexão>"}'
   - 200: a resposta traz `server` (a URL base certa) e `ai_agent.mcp`.
   - 400: o token de conexão está malformado. A resposta explica o formato
     esperado. Me avise e pare.
   - 401: o token não é deste servidor. Me avise e pare.

3. Registre o MCP com a URL de `ai_agent.mcp` e o `access_token` da resposta:
   claude mcp add --transport http --scope user rustploy <URL_MCP> --header "Authorization: Bearer <access_token>"
   Se já existir um servidor chamado "rustploy", rode antes
   `claude mcp remove rustploy --scope user`.

4. Rode `claude mcp list` e confirme que "rustploy" aparece como Connected.

5. Faça um teste só de leitura pelo MCP via curl (JSON-RPC 2.0 com POST na URL do MCP e o mesmo Bearer):
   - `initialize`;
   - `tools/list`;
   - `tools/call` da ferramenta `status`;
   - `tools/call` da ferramenta `manual`, e leia o texto inteiro: é o manual
     de operação do servidor.
   Me resuma a versão do daemon, quantos serviços estão rodando e a fila de deploys.

Regras:
- Neste setup, não faça nada que altere o servidor: nada de deploy, delete,
  prune, stop ou ServiceUpdate.
- Quando for usar o MCP nas próximas sessões, peça minha confirmação antes de
  qualquer comando destrutivo. O token é de administrador e o servidor não
  pede confirmação.
- Avise que as ferramentas do MCP só aparecem numa sessão nova do Claude Code,
  e que dá para conferir com /mcp.
```

## Mais de um servidor

O nome `rustploy` no `claude mcp add` é só um rótulo. Para operar vários
daemons da mesma máquina, dê um nome por servidor e diga ao agente qual usar:

```bash
claude mcp add --transport http --scope user rustploy-producao https://prod.exemplo.com:9797/mcp --header "Authorization: Bearer <token A>"
claude mcp add --transport http --scope user rustploy-staging  https://stg.exemplo.com:9797/mcp  --header "Authorization: Bearer <token B>"
```

As ferramentas viram `mcp__rustploy-producao__deploy`,
`mcp__rustploy-staging__deploy` etc. Além do nome, cada daemon se identifica
no handshake. O `serverInfo.title` é `Rustploy daemon em <host:porta>`, e a
primeira linha das `instructions` diz em qual servidor o agente está e pede
confirmação antes de alterar algo quando houver mais de um registrado. O
`host:porta` é o do header `Host` pelo qual o agente chegou.

Se cada repositório de app sobe num servidor diferente, dá para amarrar o
servidor ao projeto com `--scope project`. O registro vai para um `.mcp.json`
na raiz do repo, que é commitado, então o token entra por variável de
ambiente:

```json
{
  "mcpServers": {
    "rustploy": {
      "type": "http",
      "url": "https://prod.exemplo.com:9797/mcp",
      "headers": { "Authorization": "Bearer ${RUSTPLOY_TOKEN}" }
    }
  }
}
```

## Problemas comuns

| Sintoma | Causa provável |
|---|---|
| `curl` por https dá erro de TLS, e http funciona | o daemon não tem `[api].domain`, então não termina TLS. Use `http://`. |
| `curl` por http não responde, e https funciona | o daemon tem `[api].domain` e só fala TLS nessa porta. |
| `/api/login` ou `/mcp` devolve `404` | o daemon é anterior ao MCP. Atualize o rustployd na VPS. |
| `401` no `/mcp` | o token da API foi trocado no `config.toml`. Pegue o token de conexão de novo. |
| `claude mcp list` mostra `✗ Failed to connect` | porta fechada no firewall da VPS, ou o host/esquema errado no `claude mcp add`. |
| as ferramentas não aparecem no agente | a sessão foi aberta antes do `claude mcp add`. Abra uma nova. |
| o host decodificado é um IP interno | o daemon não tem `[api].domain` e usou o IP de saída da VPS. Configure `api.domain` e reinicie. |

## Trocar ou remover

```bash
claude mcp remove rustploy --scope user   # remove
claude mcp list                           # confere o que está registrado
```

Depois de trocar o token na VPS, remova e rode o `add` de novo com o token
novo.
