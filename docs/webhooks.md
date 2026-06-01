# Webhooks de Deploy

O Rustploy expõe um endpoint HTTP que permite que plataformas externas (GitHub, GitLab, Gitea, Docker Hub, etc.) disparem um novo deploy automaticamente ao detectar um push ou publicação de imagem.

## Como funciona

Cada serviço do tipo **Application** (Registry ou Git) recebe um token aleatório de 48 caracteres hexadecimais na primeira vez que é deployado. Esse token é combinado com o ID do serviço para formar a URL de webhook:

```
POST {base_url}/webhook/{service_id}/{token}
```

O servidor valida o token e, se correto, inicia um deploy do serviço imediatamente. O corpo da requisição é **ignorado** — a autenticação é feita inteiramente pelo token na URL.

## Configuração do domínio

Por padrão a URL usa o IP de saída detectado automaticamente:

```
http://192.168.1.42:9001/webhook/01JXABC.../a3f8e2c1...
```

Para usar um domínio próprio, acesse **Settings › Web Server** no TUI e configure o campo **Domínio / URL base**:

```
https://rustploy.meusite.com
```

A URL do webhook passa a ser:

```
https://rustploy.meusite.com/webhook/01JXABC.../a3f8e2c1...
```

> O servidor de webhook escuta na porta `9001` por padrão (configurável em `[daemon] webhook_port`). Se você usar um domínio com HTTPS, configure seu proxy reverso para encaminhar o tráfego HTTPS para `localhost:9001`.

## Encontrando a URL no TUI

1. Abra um serviço Application
2. Vá para a aba **Deployments**
3. A URL aparece logo abaixo dos detalhes do último deploy:

```
  Webhook:  https://rustploy.meusite.com/webhook/01JXABC.../a3f8e2c1...
  [c] copiar  [w] regenerar token
```

Pressione **`c`** para copiar a URL para a área de transferência ou **`w`** para gerar um novo token (o anterior é invalidado imediatamente).

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

## Dependência para copiar a URL no TUI

O atalho `[c]` na aba Deployments usa ferramentas externas para acessar o clipboard do sistema. Instale uma delas conforme seu ambiente:

| Ambiente | Pacote | Comando |
|----------|--------|---------|
| Wayland (recomendado) | `wl-clipboard` | `sudo apt install wl-clipboard` |
| X11 | `xclip` | `sudo apt install xclip` |
| X11 (alternativo) | `xsel` | `sudo apt install xsel` |

Se nenhuma ferramenta estiver presente, o TUI exibirá uma notificação informando o que instalar. A URL ainda pode ser copiada manualmente da tela.

## Segurança

- O token tem 48 caracteres hexadecimais (192 bits de entropia), gerado via `/dev/urandom`
- Cada serviço tem um token único e independente
- Regenerar o token (`[w]` no TUI) invalida imediatamente o token anterior
- O endpoint não revela se um `service_id` existe — tokens inválidos sempre retornam `401`
- Recomenda-se usar HTTPS em produção para que o token não trafegue em claro

## Configuração avançada

```toml
# config.toml

[daemon]
webhook_port = 9001  # porta do servidor de webhook (default: 9001)
```

O servidor de webhook é independente do proxy reverso de aplicações — ele escuta em uma porta dedicada para não interferir com o roteamento de domínios das aplicações deployadas.
