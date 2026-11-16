# Secrets — Gerenciamento de Credenciais

Secrets permitem armazenar valores sensíveis (tokens, senhas, chaves de API) de forma criptografada e referenciá-los nas configurações dos serviços, sem que o valor real apareça em nenhuma configuração persistida.

---

## Como funciona internamente

### Armazenamento

Quando um secret é criado, o daemon criptografa o valor usando `age` antes de gravá-lo no banco de dados SQLite. A identidade `age` (chave privada do servidor) fica em `/etc/rustploy/age.key` (`[secrets] age_key_path`) ou inline em `[secrets] age_identity`.

O que fica no banco de dados:
- **project_id** — a qual projeto pertence
- **name** — o nome do secret (ex: `MEU_TOKEN`)
- **encrypted_blob** — o valor criptografado; o texto real nunca é gravado em disco

### Tipos de referência

Há dois lugares onde um secret pode ser referenciado:

| Onde | Campo | O que guardar | Para que serve |
|------|-------|---------------|----------------|
| Aba General do serviço | **Credentials** | nome do secret, puro | autenticação Git no clone |
| Variáveis do projeto / Environment do serviço | **valor** | `<secret:NOME>` | variável de ambiente no container |

---

## Caso 1: Repositório Git privado (GitHub, GitLab, etc.)

Este é o fluxo correto para clonar repos privados via HTTPS.

### Passo 1 — Criar o secret

Na tela do projeto, aba **Secrets**:

- **NOME**: `GITHUB_TOKEN` (ou qualquer nome)
- **valor**: o Personal Access Token do GitHub (ex: `ghp_xxxxxxxxxxxxxxxx`)

Salvar cifra o valor na hora. Ele não volta a ser exibido em lugar nenhum: a
lista mostra só o nome. Para trocar o valor, salve outro com o **mesmo nome** —
sobrescreve.

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

Aba **Secrets** do projeto:

- **NOME**: `API_KEY`
- **valor**: o valor real da chave

### Passo 2 — Referenciar na env var

Há três caminhos, todos equivalentes (todos gravam `EnvVarValue::Secret`):

- **Formulário de adicionar variável** (projeto ou serviço) — marque **"usar
  secret"** e escolha o secret na lista que toma o lugar do campo do valor. Na
  variável de um serviço a lista traz os secrets do serviço e os do projeto
  (marcados "(projeto)"); na do projeto, só os do projeto; na de um job
  (janela "Variáveis do job", aberta pela aba Jobs do projeto ou pelo editar
  job em Agendamentos), os do job, os do serviço gatilho (marcados "(serviço)")
  e os do projeto. Um nome que um escopo de cima já tem fica de fora da lista,
  porque é o de cima que vale.
- **Numa variável que já existe** — marque **"usar secret"** na própria linha e
  escolha o secret; o **OK** da linha pede confirmação e grava. Desmarcar volta
  a variável para um valor em claro (digitado na linha).
- **Editor `.env`** (projeto ou serviço) — a linha `API_KEY=<secret:API_KEY>`.
  Escrever `<secret:API_KEY>` ou `secret:API_KEY` como valor no formulário de
  adicionar também funciona.

Depois de salvar, a linha mostra **"usar secret"** marcado e o nome do secret na
lista. Um secret que a variável usa mas que foi apagado aparece na lista como
"(não encontrado)".

A chave e o valor de cada linha se editam no lugar: clicar no campo já edita, e
nada grava sozinho — o **OK** aparece quando a linha muda, pede confirmação e
grava; o **✕** ao lado desfaz.

### Como o deploy usa isso

O executor itera as env vars do serviço:
- `Plain("valor")` → passa direto para o Docker
- `Secret("API_KEY")` → busca no banco, descriptografa em memória, passa para o Docker

O container recebe a variável `API_KEY` com o valor real, sem que ele apareça na especificação do serviço.

---

## Remover um secret

Botão `✕` na linha do secret (aba Secrets do projeto). Não há como recuperar o
valor depois, e as variáveis que referenciam aquele nome passam a receber string
vazia no próximo deploy — a resolução usa `unwrap_or_default()`, então o deploy
não falha, o container só sobe sem o valor.

---

## O que é e o que não é protegido

**Protegido:**
- O valor nunca é gravado em texto plano no banco
- Listar secrets retorna apenas os nomes, nunca os valores
- O valor descriptografado existe apenas em memória RAM, na hora do deploy
- URLs de clone têm as credenciais substituídas por `***` nos logs

**Limitações:**
- Um usuário com acesso ao host pode inspecionar env vars de containers via `docker inspect`
- Comprometer `/etc/rustploy/age.key` expõe todos os secrets de todos os projetos
- Não há rotação automática; revogar um token requer deletar o secret, recriar e fazer novo deploy
