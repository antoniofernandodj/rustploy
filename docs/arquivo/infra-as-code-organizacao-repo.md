# Infra-as-Code — Onde versionar os manifestos

Guia de decisão para responder: **em qual repositório Git deve viver o `rustploy.yml`?**
No repo da API? No frontend? Num projeto separado só de infra?

> Pré-requisito: leia [infra-as-code.md](infra-as-code.md) para o formato do manifesto e os comandos `apply`/`export`.

---

## O critério que decide

A pergunta real é: **a config de deploy muda junto com o código, ou tem vida própria?**

- Coisas como `port`, `domain`, `env`, `healthcheck` mudam **raramente** e descrevem *o servidor*.
- O código muda **o tempo todo**.

Cadências de mudança diferentes geralmente pedem repositórios diferentes — mas há nuances que dependem do acoplamento que você quer.

---

## As opções

### ❌ Tudo no repo da API (o "app mais importante")

É o atalho mais comum, mas acopla o ciclo de vida de **todos** os serviços ao repo da API. O `db`, o `frontend`, o `redis` passam a depender de commits num repo que não é deles. Só vale a pena se você literalmente tem **um serviço só**.

### ❌ No frontend

Não há motivo técnico. O frontend é apenas mais um serviço; não tem nada que o qualifique como dono da infra.

### ✅ Repo separado só de infra — *recomendado para single-node*

Um repositório `infra/` (ou `deploy/`) com um `stack.yml` raiz que agrega tudo. Você ganha:

- **Visão única do servidor inteiro** — todo o estado declarado num lugar, versionado.
- **Um lugar só para rodar** `rustploy apply -f stack.yml --prune --deploy`.
- **Um lugar só para os segredos de ops** (o `.env` não-versionado que alimenta os `${VAR}`).
- `--prune` faz sentido de verdade aqui: o `stack.yml` vira a **fonte da verdade** do nó.

### ✅ `rustploy.yml` em cada repo de app — *quando a config é "do app"*

Se uma variável nova *nasce* junto com uma mudança de código (o mesmo dev que mexe no app também mexe no spec), faz sentido o `rustploy.yml` morar no repositório do app, ao lado do código.

---

## O detalhe técnico que muda a decisão

O `include:` do manifesto raiz resolve **caminho relativo local** — ele **não** clona repositório remoto. Então, para combinar "um `rustploy.yml` por app" **com** um "`stack.yml` agregador", o repo de infra precisa ter esses arquivos **no mesmo checkout** na hora do `apply`. Na prática, duas formas:

- **git submodules** apontando para os repos dos apps; ou
- um **passo de CI** no repo de infra que clona os apps antes de rodar o `apply`.

Sem isso, um agregador com `include:` apontando para outro repositório **não funciona** — os arquivos precisam estar lado a lado em disco no momento do apply.

---

## Recomendação concreta

Para o Rustploy single-node, na maioria das escalas, **comece simples com um repositório de infra dedicado**, com os projetos inline (sem `include:` cross-repo):

```
infra/
├── stack.yml          # manifesto raiz: todos os projetos do nó
├── .env.example       # documenta os ${VAR} necessários (sem valores reais)
├── .gitignore         # ignora .env — segredos reais nunca entram no Git
└── README.md          # "como aplicar": rustploy apply -f stack.yml --prune --deploy
```

Regras de ouro para esse repo:

- **Segredos de verdade** → `secret:NOME` no YAML, cadastrados via TUI/`SecretSet`. O valor **nunca** entra no Git. Veja [secrets.md](secrets.md).
- **Config sensível-mas-não-secreta** → `${VAR}` no YAML + `.env` local **não-versionado** (no `.gitignore`).
- **Deploy de código** continua vindo dos **webhooks** (push → rebuild). Veja [webhooks.md](webhooks.md).
- **`apply`** você roda quando muda a *forma* do serviço: porta, domínio, env, novo serviço, remoção de serviço.

### Quando migrar para o modelo "per-app + submodules"

Evolua para `rustploy.yml` em cada repo de app (com submodules ou CI montando o checkout) **somente quando**:

- os apps tiverem **donos/times diferentes**; ou
- a config de deploy começar a mudar **junto com o código** com frequência.

Antes disso, é complexidade sem retorno.

---

## Fluxo de trabalho sugerido

```bash
# 1. (uma vez) partir do estado atual do servidor, se já existir
rustploy export minha-api   -o infra/api.yml
rustploy export meu-front   -o infra/front.yml
# montar o stack.yml raiz com include: ./api.yml etc. (ou colar inline)

# 2. versionar
cd infra && git init && git add . && git commit -m "infra inicial"

# 3. no dia a dia: editar o YAML, revisar no PR, aplicar
DB_PASS=... rustploy apply -f stack.yml --prune --deploy
```

- O **PR** vira o ponto de revisão das mudanças de infra (diff de porta, env, domínio...).
- `--prune` garante que remover um serviço do YAML o remove do servidor.
- `--deploy` faz o rollout dos serviços alterados na mesma chamada (GitOps completo).

---

## Resumo

| Onde | Veredito |
|------|----------|
| Repo da API | ❌ só se houver um único serviço |
| Repo do frontend | ❌ sem motivo |
| **Repo de infra dedicado** | ✅ **recomendado** — comece inline |
| `rustploy.yml` por app + submodules/CI | ✅ quando times/cadência justificarem |

**Padrão**: repositório de infra separado, projetos inline, segredos fora do Git. Migre para per-app só quando a organização exigir.
