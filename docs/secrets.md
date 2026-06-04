# Secrets — Gerenciamento de Credenciais

Secrets permitem armazenar valores sensíveis (tokens, senhas, chaves de API) de forma criptografada e referenciá-los nas configurações dos serviços, sem que o valor real apareça em nenhuma configuração persistida.

---

## Como funciona internamente

### Armazenamento

Quando um secret é criado, o daemon criptografa o valor usando `age` antes de gravá-lo no banco de dados SQLite. A chave mestre fica em `/etc/rustploy/master.key`.

O que fica no banco de dados:
- **project_id** — a qual projeto pertence
- **name** — o nome do secret (ex: `MEU_TOKEN`)
- **encrypted_blob** — o valor criptografado; o texto real nunca é gravado em disco

### Tipos de referência

Há dois lugares onde um secret pode ser referenciado:

| Onde | Campo | O que guardar | Para que serve |
|------|-------|---------------|----------------|
| Aba General do serviço | **Credentials** | nome do secret | autenticação Git no clone |
| Aba Environment do serviço | **VALUE** com prefixo `secret:` | `secret:NOME` | variável de ambiente no container |

---

## Caso 1: Repositório Git privado (GitHub, GitLab, etc.)

Este é o fluxo correto para clonar repos privados via HTTPS.

### Passo 1 — Criar o secret

Na tela do projeto, aba **Secrets** → tecla `n`:

- **NAME**: `GITHUB_TOKEN` (ou qualquer nome)
- **VALUE**: o Personal Access Token do GitHub (ex: `ghp_xxxxxxxxxxxxxxxx`)

O sistema monta a URL de clone no formato:
```
https://x-token-auth:{token}@github.com/usuario/repo.git
```
`x-token-auth` é o username fixo usado pelo sistema. **Para GitHub.com isso funciona** — o GitHub ignora o username e valida apenas o PAT. Para outros provedores (GitLab self-hosted, Bitbucket Server), pode ser necessário o username real; isso ainda não é suportado pelo campo Credentials.

### Passo 2 — Configurar o serviço

Na aba **General** do serviço:

- **Repo URL**: `https://github.com/usuario/repo.git` (obrigatório HTTPS, não SSH)
- **Credentials**: `GITHUB_TOKEN` ← o **nome** do secret, não o valor

O campo Credentials aceita o nome do secret diretamente (sem prefixo `secret:`).

### Como o deploy usa isso

Na hora do clone, o executor:
1. Lê o campo `credentials` do serviço (ex: `"GITHUB_TOKEN"`)
2. Busca o secret no banco pelo nome
3. Descriptografa o valor em memória
4. Injeta na URL: `https://x-token-auth:{token}@github.com/usuario/repo.git`
5. Faz o clone com a URL autenticada

O token existe apenas em memória RAM durante o clone. Não aparece em logs (a URL é exibida com `***`).

> **Atenção**: URLs SSH (`git@github.com:...`) são passadas sem modificação — o campo Credentials não tem efeito com SSH.

---

## Caso 2: Variáveis de ambiente secretas no container

Para passar um secret como variável de ambiente para a aplicação em execução (ex: uma API key que a aplicação lê via `std::env::var`).

### Passo 1 — Criar o secret

Aba **Secrets** do projeto → `n`:

- **NAME**: `API_KEY`
- **VALUE**: o valor real da chave

### Passo 2 — Referenciar na env var do serviço

Aba **Environment** do serviço → `n`:

- **KEY**: `API_KEY`
- **VALUE**: `secret:API_KEY`

O valor exibido após salvar será `<secret:API_KEY>`, indicando que é uma referência.

### Como o deploy usa isso

O executor itera as env vars do serviço:
- `Plain("valor")` → passa direto para o Docker
- `Secret("API_KEY")` → busca no banco, descriptografa em memória, passa para o Docker

O container recebe a variável `API_KEY` com o valor real, sem que ele apareça na especificação do serviço.

---

## Referência rápida de teclas (aba Secrets)

| Tecla | Ação |
|-------|------|
| `n` | Novo secret |
| `D` | Remover secret selecionado |
| `↑` / `↓` | Navegar entre secrets |
| `Tab` | Alternar campo NAME ↔ VALUE ao criar |
| `Enter` | Salvar |
| `Esc` | Cancelar |

---

## O que é e o que não é protegido

**Protegido:**
- O valor nunca é gravado em texto plano no banco
- Listar secrets retorna apenas os nomes, nunca os valores
- O valor descriptografado existe apenas em memória RAM, na hora do deploy
- URLs de clone têm as credenciais substituídas por `***` nos logs

**Limitações:**
- Um usuário com acesso ao host pode inspecionar env vars de containers via `docker inspect`
- Comprometer `/etc/rustploy/master.key` expõe todos os secrets de todos os projetos
- Não há rotação automática; revogar um token requer deletar o secret, recriar e fazer novo deploy
