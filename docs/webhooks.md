# Webhooks de Deploy

O Rustploy expõe um endpoint HTTP que permite que plataformas externas (GitHub, GitLab, Gitea, Docker Hub, etc.) disparem um novo deploy automaticamente ao detectar um push ou publicação de imagem.

## Como funciona

Cada serviço do tipo **Application** (Registry ou Git) recebe um token aleatório de 48 caracteres hexadecimais na primeira vez que é deployado. Esse token é combinado com o ID do serviço para formar a URL de webhook:

```
POST {base_url}/webhook/{service_id}/{token}
```

O servidor valida o token e, se correto, inicia um deploy do serviço imediatamente. O corpo da requisição é **ignorado** — a autenticação é feita inteiramente pelo token na URL.

## Uma porta só: a da API

O webhook é servido pelo **mesmo listener HTTP da API** (`[api] port`, default
`9797`) — não há mais porta dedicada. O path `/webhook/...` é liberado **antes**
da checagem do Bearer token da API, porque a autenticação dele é o token na
própria URL.

Consequência prática: **para o GitHub (ou Gitea, ou Docker Hub) alcançar o
webhook, a porta da API precisa estar acessível para eles.** Como o default é
bind em `127.0.0.1`, isso significa configurar `[api] bind_address` (com
`token`, obrigatório fora do loopback) ou publicar a API via proxy.

## Configuração do domínio

A base da URL é **derivada** da config `[api]`, não configurada à parte:

- Com `[api] domain` definido, o próprio listener termina TLS (cert automático
  via ACME) e a base é `https://<domain>` — mais `:<port>`, quando a porta não é
  a 443:

  ```
  https://rustploy.meusite.com:9797/webhook/01JXABC.../a3f8e2c1...
  ```

- Sem domínio, a base usa o IP de saída detectado automaticamente:

  ```
  http://192.168.1.42:9797/webhook/01JXABC.../a3f8e2c1...
  ```

A URL efetiva aparece pronta na GUI (**Settings › Web Server** mostra a base
derivada; a aba Deployments do serviço mostra a URL completa).

## Encontrando a URL na GUI

1. Abra um serviço **Application** (Git ou Registry — Compose não tem webhook)
2. Vá para a aba **Deployments**
3. A URL aparece no topo, com **Copiar** e **Regenerar**

O token só existe a partir do **primeiro deploy** do serviço; antes disso a aba
avisa que é preciso rodar um deploy. **Regenerar** invalida a URL anterior
imediatamente — recadastre-a no provedor.

## Formato da requisição

```
POST /webhook/{service_id}/{token}
```

**O corpo da requisição não é lido pelo Rustploy.** A autenticação é feita inteiramente pelo token na URL — qualquer body (ou nenhum) é aceito. Isso significa que você pode chamar o endpoint diretamente de um GitHub Webhook, de um GitLab Webhook, de um GitHub Action, ou de um script simples sem precisar formatar um payload específico.

### Trigger manual (sem body)

```bash
curl -X POST https://rustploy.meusite.com/webhook/01JXABC.../a3f8e2c1...
```

### Trigger manual (com body JSON arbitrário)

O Rustploy ignora o body, mas algumas plataformas exigem `Content-Type` definido:

```bash
curl -X POST https://rustploy.meusite.com/webhook/01JXABC.../a3f8e2c1... \
  -H "Content-Type: application/json" \
  -d '{"ref": "refs/heads/main"}'
```

### O que o GitHub envia

Quando um GitHub Webhook dispara, ele envia um `POST` com um payload JSON extenso. O Rustploy recebe e **descarta** esse body — só o token na URL importa. Para referência, o body de um evento `push` do GitHub tem esta estrutura:

```json
{
  "ref": "refs/heads/main",
  "before": "abc123...",
  "after": "def456...",
  "repository": {
    "id": 123456789,
    "full_name": "usuario/repositorio",
    "clone_url": "https://github.com/usuario/repositorio.git"
  },
  "pusher": {
    "name": "usuario",
    "email": "usuario@example.com"
  },
  "commits": [
    {
      "id": "def456...",
      "message": "fix: corrige bug na autenticação",
      "author": { "name": "Usuario", "email": "usuario@example.com" }
    }
  ]
}
```

O GitHub também envia cabeçalhos adicionais (`X-GitHub-Event: push`, `X-Hub-Signature-256`, etc.) que são igualmente ignorados.

### O que o GitLab envia

```json
{
  "object_kind": "push",
  "ref": "refs/heads/main",
  "before": "abc123...",
  "after": "def456...",
  "project": {
    "id": 123,
    "name": "repositorio",
    "http_url": "https://gitlab.com/usuario/repositorio"
  },
  "commits": [
    {
      "id": "def456...",
      "message": "fix: corrige bug",
      "author": { "name": "Usuario", "email": "usuario@example.com" }
    }
  ]
}
```

### O que o Gitea envia

O Gitea foi projetado para ser compatível com a API e os webhooks do GitHub — o payload de push é **idêntico ao do GitHub**. Não é necessário tratar os dois de forma diferente.

### O que o Docker Hub envia

O Docker Hub envia uma notificação quando uma nova tag de imagem é publicada:

```json
{
  "push_data": {
    "pushed_at": 1690000000,
    "pusher": "usuario",
    "tag": "latest"
  },
  "repository": {
    "name": "minha-imagem",
    "namespace": "usuario",
    "repo_name": "usuario/minha-imagem",
    "status": "Active"
  }
}
```

> **Nota sobre uso futuro:** o Rustploy atualmente não parseia o body do webhook. Em versões futuras, o payload poderá ser usado para filtrar deploys por branch (ex: deployar apenas quando `ref == "refs/heads/main"`) ou para registrar qual commit disparou o deploy.

## Testando com curl

```bash
curl -X POST https://rustploy.meusite.com/webhook/01JXABC.../a3f8e2c1...
```

Resposta em caso de sucesso (`200 OK`):

```
deploy triggered
```

Respostas de erro:

| Status | Motivo |
|--------|--------|
| `401 Unauthorized` | Token inválido ou serviço não encontrado |
| `404 Not Found` | Path incorreto |
| `405 Method Not Allowed` | Método diferente de POST |

## Integrando com GitHub

1. No repositório, vá em **Settings › Webhooks › Add webhook**
2. **Payload URL**: cole a URL do webhook
3. **Content type**: `application/json` (o corpo é ignorado pelo Rustploy, mas o GitHub exige um valor)
4. **Secret**: deixe em branco — a autenticação é feita pelo token na URL
5. **Which events**: marque apenas **Just the push event** (ou os eventos que fizerem sentido)
6. Marque **Active** e clique em **Add webhook**

A partir daí, cada push para o repositório dispara automaticamente um deploy no Rustploy.

### Filtrando por branch (limitação atual)

O Rustploy ainda não filtra por branch — qualquer push aciona o deploy independentemente do branch. Se precisar filtrar, use um intermediário como um GitHub Action que chame o webhook apenas em pushes para `main`:

```yaml
# .github/workflows/deploy.yml
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Trigger deploy
        run: curl -X POST ${{ secrets.RUSTPLOY_WEBHOOK_URL }}
```

## Integrando com GitLab

1. No projeto, vá em **Settings › Webhooks**
2. **URL**: cole a URL do webhook
3. **Secret token**: deixe em branco
4. Marque **Push events**
5. Clique em **Add webhook**

## Integrando com Gitea

1. No repositório, vá em **Settings › Webhooks › Add Webhook › Gitea**
2. **Target URL**: cole a URL do webhook
3. **HTTP Method**: `POST`
4. **POST Content Type**: `application/json`
5. Marque os eventos desejados e clique em **Add Webhook**

## Integrando com Docker Hub

O Docker Hub envia um POST quando uma nova imagem é publicada em um repositório.

1. No repositório Docker Hub, vá em **Webhooks**
2. Dê um nome e cole a URL do webhook
3. Clique em **Create**

Útil para serviços do tipo **Registry**: quando você publica uma nova versão da imagem, o Rustploy faz pull automaticamente e reinicia o container.

## Segurança

- O token tem 48 caracteres hexadecimais (192 bits de entropia), gerado via `/dev/urandom`
- Cada serviço tem um token único e independente
- A comparação do token é feita em tempo constante
- Regenerar o token (botão **Regenerar**, na aba Deployments) invalida imediatamente o token anterior
- O endpoint não revela se um `service_id` existe — tokens inválidos sempre retornam `401`
- O `/webhook/...` ser público **não** abre a API: todo o resto (`/api/*`) continua exigindo o Bearer token
- Recomenda-se usar HTTPS em produção (`[api] domain`) para que o token não trafegue em claro

## Configuração

```toml
# config.toml

[api]
port         = 9797                    # o webhook é servido nesta MESMA porta
bind_address = "0.0.0.0"               # precisa ser alcançável pelo provedor
token        = "<token forte>"          # obrigatório fora do loopback
domain       = "rustploy.meusite.com"  # opcional: HTTPS automático (ACME) aqui
```

Não existe mais `[daemon] webhook_port` — a porta dedicada (8788) foi eliminada
na unificação (ver `docs/plano-unificacao-webhook-api.md`). Um `config.toml`
antigo que ainda declare o campo continua carregando: a chave é simplesmente
ignorada.
