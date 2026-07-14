# Webhooks: exemplos de requisição e resposta

Complemento prático de [`webhooks.md`](webhooks.md). Tudo aqui foi **capturado de
um daemon real** (`curl -i` contra um serviço `nginx:alpine` deployado), não
escrito de memória.

A URL tem sempre esta forma — pegue a sua na GUI, aba **Deployments** do serviço
(botão **Copiar**):

```
POST {base}/webhook/{service_id}/{token}
```

Onde `{base}` é a URL pública da API (mesma porta — default `9797`), derivada de
`[api] domain`/`port`.

## A regra que resume tudo

**O corpo da requisição é lido e descartado.** Não há assinatura HMAC, não há
secret, não há campo obrigatório: quem autentica é o token de 192 bits na URL.
Qualquer body (ou nenhum) produz exatamente o mesmo resultado — o que é
justamente o que permite plugar GitHub, GitLab, Gitea, Docker Hub ou um `curl`
solto sem adaptar payload nenhum.

Corolário: **trate a URL como uma senha.** Quem a tiver, faz deploy.

---

## 1. Disparo manual (sem corpo)

O caso mais simples, e o que você usa para testar:

```bash
curl -i -X POST \
  https://rustploy.meusite.com:9797/webhook/svc_01KXHB19G5GR/3a7ebcd0bee83568…
```

Resposta:

```http
HTTP/1.1 200 OK
content-length: 16

deploy triggered
```

## 2. Push do Gitea — o corpo completo (integração principal)

Este é o caso que mais importa aqui. Quando você faz `git push` num repositório
do Gitea que tem o webhook cadastrado, ele monta um `POST` com **estes headers**:

```http
POST /webhook/svc_01KXHB19G5GR/3a7ebcd0bee83568… HTTP/1.1
Host: rustploy.meusite.com:9797
Content-Type: application/json
User-Agent: Gitea/1.22.0
X-Gitea-Event: push
X-Gitea-Event-Type: push
X-Gitea-Delivery: f6266f16-1bf3-46a5-9ea4-602e06ead473
X-Gitea-Signature: 9f3e1b…            (só quando você preenche um Secret)
X-Gogs-Event: push                     (aliases de compatibilidade — o Gitea
X-GitHub-Event: push                    manda os três conjuntos)
```

E **este corpo** (`PushPayload`, de `modules/structs/hook.go`) — completo, com o
que a versão atual do Gitea envia:

```json
{
  "ref": "refs/heads/main",
  "before": "28e1879d029cb852e4844d9c718537df08844e03",
  "after": "bffeb74224043ba2feb48d137756c8a9331c449a",
  "compare_url": "https://git.meusite.com/antonio/landing-page/compare/28e1879d029cb852e4844d9c718537df08844e03...bffeb74224043ba2feb48d137756c8a9331c449a",
  "total_commits": 1,
  "commits": [
    {
      "id": "bffeb74224043ba2feb48d137756c8a9331c449a",
      "message": "fix: corrige o link do rodapé\n",
      "url": "https://git.meusite.com/antonio/landing-page/commit/bffeb74224043ba2feb48d137756c8a9331c449a",
      "author": {
        "name": "Antonio Fernando",
        "email": "antonio@meusite.com",
        "username": "antonio"
      },
      "committer": {
        "name": "Antonio Fernando",
        "email": "antonio@meusite.com",
        "username": "antonio"
      },
      "verification": {
        "verified": false,
        "reason": "gpg.error.not_signed_commit",
        "signature": "",
        "signer": null,
        "payload": ""
      },
      "timestamp": "2026-07-14T22:13:41-03:00",
      "added": [],
      "removed": [],
      "modified": ["templates/footer.html"]
    }
  ],
  "head_commit": {
    "id": "bffeb74224043ba2feb48d137756c8a9331c449a",
    "message": "fix: corrige o link do rodapé\n",
    "url": "https://git.meusite.com/antonio/landing-page/commit/bffeb74224043ba2feb48d137756c8a9331c449a",
    "author": {
      "name": "Antonio Fernando",
      "email": "antonio@meusite.com",
      "username": "antonio"
    },
    "committer": {
      "name": "Antonio Fernando",
      "email": "antonio@meusite.com",
      "username": "antonio"
    },
    "verification": {
      "verified": false,
      "reason": "gpg.error.not_signed_commit",
      "signature": "",
      "signer": null,
      "payload": ""
    },
    "timestamp": "2026-07-14T22:13:41-03:00",
    "added": [],
    "removed": [],
    "modified": ["templates/footer.html"]
  },
  "repository": {
    "id": 140,
    "owner": {
      "id": 1,
      "login": "antonio",
      "full_name": "Antonio Fernando",
      "email": "antonio@meusite.com",
      "avatar_url": "https://git.meusite.com/avatars/1",
      "username": "antonio"
    },
    "name": "landing-page",
    "full_name": "antonio/landing-page",
    "description": "",
    "private": false,
    "fork": false,
    "html_url": "https://git.meusite.com/antonio/landing-page",
    "ssh_url": "ssh://git@git.meusite.com:2222/antonio/landing-page.git",
    "clone_url": "https://git.meusite.com/antonio/landing-page.git",
    "website": "",
    "stars_count": 0,
    "forks_count": 0,
    "watchers_count": 1,
    "open_issues_count": 0,
    "default_branch": "main",
    "created_at": "2026-02-26T04:29:06-03:00",
    "updated_at": "2026-07-14T22:13:41-03:00"
  },
  "pusher": {
    "id": 1,
    "login": "antonio",
    "full_name": "Antonio Fernando",
    "email": "antonio@meusite.com",
    "avatar_url": "https://git.meusite.com/avatars/1",
    "username": "antonio"
  },
  "sender": {
    "id": 1,
    "login": "antonio",
    "full_name": "Antonio Fernando",
    "email": "antonio@meusite.com",
    "avatar_url": "https://git.meusite.com/avatars/1",
    "username": "antonio"
  }
}
```

E a resposta do rustploy para esse corpo inteiro:

```http
HTTP/1.1 200 OK
content-length: 16

deploy triggered
```

**Exatamente a mesma** do `curl -X POST` sem corpo nenhum da seção 1 — porque
nada disso é lido. Vale reforçar três consequências disso:

- **`X-Gitea-Signature` não é verificada.** Se você preencher o campo *Secret* no
  Gitea, ele assina o corpo (HMAC-SHA256) e manda no header — o rustploy ignora.
  Deixe o *Secret* em branco; ele não te protege aqui, quem protege é o token
  secreto na URL.
- **Versões antigas do Gitea (herança do Gogs) mandavam o `secret` dentro do
  próprio JSON.** Se o seu Gitea fizer isso, o valor viaja no corpo e também é
  descartado — mais um motivo para deixar o campo vazio.
- **`ref` não filtra nada.** Um push em `refs/heads/qualquer-coisa` dispara o
  deploy igual a um push em `main` (veja o filtro por branch na seção 6).

### Reproduzindo esse push com curl

Para testar sem precisar dar `git push`, salve o JSON acima em `push.json` e:

```bash
curl -i -X POST \
  https://rustploy.meusite.com:9797/webhook/svc_01KXHB19G5GR/3a7ebcd0bee83568… \
  -H "Content-Type: application/json" \
  -H "X-Gitea-Event: push" \
  --data-binary @push.json
```

## 3. Push do GitHub, GitLab, Docker Hub

Mesmo desfecho, sem nenhum tratamento especial: o GitHub manda `ref`/`after`/
`repository`/`pusher` com `X-GitHub-Event` e `X-Hub-Signature-256`; o GitLab manda
`object_kind: "push"`; o Docker Hub manda `push_data`/`repository`. Todos recebem
`200 deploy triggered`.

Testei com um payload do GitHub carregando `X-Hub-Signature-256: sha256=deadbeef`
— uma assinatura deliberadamente falsa — e a requisição passou do mesmo jeito, o
que confirma na prática que assinatura de payload não é verificada em nenhum
provedor.

## 4. Erros

| Situação | Resposta | Corpo |
|----------|----------|-------|
| Token errado | `401 Unauthorized` | `invalid token` |
| `service_id` inexistente | `401 Unauthorized` | `invalid token` |
| Serviço nunca deployado (token ainda não existe) | `401 Unauthorized` | `invalid token` |
| `GET` na URL (ex.: colada no navegador) | `405 Method Not Allowed` | `method not allowed` |
| Path incompleto (`/webhook/soisso`) | `404 Not Found` | `not found` |

Repare que **serviço inexistente e token errado dão a mesma resposta**, de
propósito: o endpoint não confirma se um `service_id` existe.

E note o `405` no `GET`: colar a URL no navegador **não** dispara deploy — o
navegador faz `GET`, e só `POST` dispara. É o comportamento correto (senão
qualquer preview de link, antivírus ou crawler que abrisse a URL deployaria o
seu serviço).

## 5. O detalhe que engana: `200` significa "recebido", não "deployou"

O `200 deploy triggered` diz que **o token era válido e o pedido foi aceito**. O
deploy em si roda de forma assíncrona e ainda pode ser **descartado** — o caso
mais comum é já haver um deploy em andamento para aquele serviço:

```
$ curl -X POST <url do webhook>   # push 1
deploy triggered
$ curl -X POST <url do webhook>   # push 2, segundos depois
deploy triggered                  # ← mesma resposta…
```

…mas no log do daemon:

```
INFO webhook: disparando deploy
INFO deploy já em andamento          ← o segundo foi recusado aqui
```

e o histórico do serviço mostra **um** deployment novo, não dois. Ou seja: dois
pushes em sequência rápida não empilham dois deploys; o segundo é ignorado
enquanto o primeiro não termina.

**Consequência prática:** para saber se o deploy realmente aconteceu, olhe a aba
**Deployments** do serviço (ou os logs do daemon) — não o status HTTP do webhook.
Se você automatiza um `curl` em CI e precisa de certeza, confirme pela API
(`DeployHistory`), não pelo `200`.

## 6. Cadastrando nos provedores

Em todos, o segredo/secret fica **em branco** e o content type é indiferente:

- **Gitea** — no repositório: *Settings › Webhooks › Add Webhook › Gitea*.
  *Target URL* = a URL copiada da aba Deployments; *HTTP Method* `POST`;
  *POST Content Type* `application/json`; **Secret vazio**; *Trigger On* =
  *Push Events*; marque *Active*. Depois de salvar, use o botão **Test Delivery**
  do próprio Gitea: ele manda um push de teste e mostra a resposta — você deve
  ver `200` e `deploy triggered` na aba *Recent Deliveries*.
- **GitHub** — Settings › Webhooks › Add webhook. *Payload URL* = a URL copiada;
  *Content type* `application/json`; *Secret* vazio; marque *Just the push event*.
- **GitLab** — Settings › Webhooks. *URL* = a URL copiada; *Secret token* vazio;
  marque *Push events*.
- **Docker Hub** — repositório › Webhooks. Útil para serviços do tipo Registry:
  publicou imagem nova, o rustploy puxa e reinicia.

> **Filtro por branch:** o rustploy **não** filtra — como ele não lê o corpo,
> qualquer push dispara o deploy, venha de que branch vier. Se você precisa
> deployar só no `main`, deixe o filtro no CI e chame o webhook de lá:
>
> ```yaml
> # .github/workflows/deploy.yml
> on:
>   push:
>     branches: [main]
> jobs:
>   deploy:
>     runs-on: ubuntu-latest
>     steps:
>       - run: curl -fsS -X POST ${{ secrets.RUSTPLOY_WEBHOOK_URL }}
> ```
>
> (`-f` faz o `curl` sair com erro se o rustploy responder 401/404 — sem ele, um
> token revogado passaria despercebido, com o job verde.)

## 7. Regenerar o token

O botão **Regenerar** (aba Deployments) emite um token novo e **invalida o
anterior na hora**: a URL antiga passa a responder `401 invalid token`. Depois de
regenerar, recadastre a URL em todo provedor que a usava.
