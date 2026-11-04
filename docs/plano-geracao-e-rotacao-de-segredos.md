# Geração e rotação de segredos (senha de banco, broker, storage, token JWT, par VAPID)

> **Estado: SÓ PROJETADO, não implementado.** Plano à parte, ainda não fechado: o desenho abaixo nasceu dentro de
> `docs/plano-credenciais-cifradas-e-storage.md` e foi tirado de lá porque **rotacionar é mais amplo que gerar**. Antes de
> implementar é preciso fechar as perguntas das seções 2 e 3 e reescrever isto como plano de fases.

## 1. O que "rotacionar a senha do banco" realmente é


Gerar um valor novo é só o primeiro passo. O ato inteiro é:

1. **gerar** a senha nova (G1);
2. **trocá-la no sistema de fora** — o Postgres, o RabbitMQ, o Redis, o RustFS/MinIO — com um comando específico de cada motor;
3. **atualizar o rustploy**: o secret, e as variáveis/URLs que apontam para ele (`DATABASE_URL` etc.);
4. **redeployar** os serviços que usam o valor (o `ServiceReload` só reinicia o mesmo container e **não** relê env/secrets);
5. **tratar a falha no meio**: se o passo 2 deu certo e o 4 não, o banco já tem a senha nova e os apps ainda têm a velha.

Hoje isso é feito à mão. No Chiquitos (2026-11-03) foi: job com `ALTER USER`, troca do secret, redeploy, e sobrou
um `POSTGRES_PASSWORD_OLD` que ainda precisa ser apagado.


## 2. O desenho feito até agora (nascido como Fase G do plano de credenciais)

> **Por que existe.** Na Fase F precisei trocar a senha do Postgres. O agente não pode escolher a senha (ele a veria) e o
> daemon **não tinha** como gerar um segredo avulso. Improvisei: criei um banco descartável pelo assistente só para ganhar uma
> senha gerada, movi o secret e apaguei o serviço. **Funcionou, mas é uma gambiarra**: depende de um efeito colateral do
> assistente, e eu a tinha escrito no manual do agente como se fosse receita. O dono rejeitou, com razão. Este é o recurso certo.
> (A gambiarra foi removida do manual e das docs; ver `docs/release.md`, v0.1.36.)

**O problema, dividido em quatro casos** (cada um pede coisa diferente):

| # | Caso | Exemplo | O que precisa |
|---|---|---|---|
| 1 | Valor aleatório **avulso** | `JWT_SECRET`, `POSTGRES_PASSWORD`, token de API interno | gerar no daemon, guardar cifrado, **nunca devolver** |
| 2 | **Par de chaves** | VAPID (P-256), Ed25519, chave `age` | gerar as duas metades; a pública pode ser lida, a privada não |
| 3 | **Rotação** | trocar a senha do Postgres sem derrubar nada | gerar um valor novo **e** guardar o anterior por um tempo |
| 4 | **Declarativo** | "este projeto precisa de um `JWT_SECRET`" no YAML | gerar **só se faltar**, sem nunca sobrescrever |

**Os comandos (todos liberados ao agente, porque nenhum devolve o valor):**

- `SecretGenerate { project_id, service_id?, name, kind, length?, on_exists }` — gera e grava. `on_exists` é `fail` (padrão: recusa se
  já existe, para ninguém quebrar uma senha em uso por engano), `keep` (não faz nada se existe: é o idempotente, o "garanta que
  exista") ou `replace` (troca; para rotação, use `SecretRotate`). Resposta: `Ok` mais metadados **não sensíveis**: o tamanho e uma
  **impressão digital** (primeiros 8 hex do SHA-256), que serve para conferir "é o mesmo valor?" sem revelar nada.
- `SecretRotate { project_id, service_id?, name, kind, length? }` — numa transação: o valor atual vira `NAME_PREVIOUS` e um novo
  assume `NAME`. As variáveis que apontam para `NAME` não mudam. Resolve o passo mais chato da rotação (a dança de renomear) e
  entrega ao job de troca **as duas senhas**: `PGPASSWORD=${NAME_PREVIOUS}` e `NEWPW=${NAME}`. Depois de confirmar, `SecretDelete` do
  `_PREVIOUS`.
- `SecretGeneratePair { …, kind: Vapid | Ed25519 | AgeX25519, name }` — cria `NAME_PRIVATE` (secret) e `NAME_PUBLIC` (secret ou env em
  claro, à escolha) em uma operação. É a única forma de gerar VAPID sem ninguém ver a privada.

**Tipos (`kind`):**

| `kind` | Saída | Para quê | Observação |
|---|---|---|---|
| `Password` (padrão) | `A-Za-z0-9`, 24 | senhas de banco, broker, storage | seguro em URL, SQL e shell; mínimo 16 |
| `Hex` | `2×n` caracteres | segredo HMAC/JWT (HS256) | padrão 32 bytes = 64 hex |
| `Base64Url` | sem `=` | tokens, chaves de sessão | padrão 32 bytes |
| `Token` | `prefixo_` + `Base64Url` | token de API reconhecível | prefixo opcional, ex. `rp_` |
| `Uuid` | UUIDv4 | identificadores | não é segredo forte; só por conveniência |
| `Symbols` | inclui `!@#%…` | sistemas que **exigem** símbolo | **opt-in** e avisado: `@ : / % ?` quebram URLs, por isso o `Password` nunca os usa |

Aleatoriedade: `/dev/urandom` com amostragem por rejeição (o `generate_password` da Fase B) — é a mesma peça, só generalizada.

**Onde o recurso aparece:**

1. **Daemon + `shared`**: os três comandos, `SecretsManager::generate/rotate`, testes (comprimento, alfabeto, que a resposta **não**
   contém o valor, que `fail` não sobrescreve, atomicidade da rotação). Auditoria: quem, qual secret, quando, nunca o valor.
2. **GUI e webui**: um botão **"Gerar"** no formulário de secret (escolhe o tipo e o tamanho) e **"Rotacionar"** na linha do secret.
   Por padrão o valor **não é mostrado**; o 👁 (que já existe e é auditado) continua sendo o único jeito de uma pessoa ler.
3. **Manifesto/YAML (infra como código)**: `secrets: { JWT_SECRET: { generate: hex, bytes: 32 } }` vira um `SecretGenerate` com
   `on_exists: keep` ao aplicar. Idempotente, entra na sincronização com o git já cifrado, e é o jeito de um projeto novo nascer pronto.
4. **Wizard**: os `${password:16}` dos templates passam a usar o mesmo gerador (hoje têm o seu).
5. **Receitas de rotação prontas (G3)**: um job-modelo por motor — Postgres (`ALTER USER`), RabbitMQ (`rabbitmqctl change_password`),
   Redis, RustFS/MinIO — que o botão "Rotacionar" cria e executa usando `NAME_PREVIOUS` + `NAME`. Assim "rotacionar a senha do
   banco" vira **um clique** (ou um comando do agente), sem receita para improvisar.

**O que NÃO se resolve aqui, de propósito:** aplicar o valor novo **no sistema de fora** (o Postgres, o broker) é específico de cada
motor — por isso é um job-modelo (item 5), e não algo mágico do `SecretRotate`. E trocar o secret **não troca o ambiente de quem já
está rodando**: é preciso **redeploy** (o `ServiceReload` só reinicia o mesmo container; descobri isso rotacionando o banco do Chiquitos).
A G3 deve, por isso, terminar com o redeploy dos serviços que usam o secret.

**Alternativas pensadas e descartadas:**
- *O agente escolhe o valor com `SecretSet`* — o valor passa por ele e fica no histórico da conversa; é o que queremos evitar.
- *Serviço descartável como gerador (o que fiz)* — efeito colateral de outro recurso; quebra se o wizard mudar; confunde a auditoria.
- *Placeholder dentro do valor da env (`${random:24}`)* — gera a cada resolução (a senha mudaria a cada deploy) ou exige memorizar
  em algum lugar, que é justamente um secret. Gerar é uma ação explícita e única, não parte da leitura da variável.
- *Pedir sempre a uma pessoa* — não escala, e é o que o dono quer poder delegar com segurança.

**Fases:** G1 = `SecretGenerate`/`Rotate` + botões + testes (a parte mais valiosa, e a que destrava as outras); G2 = `Pair`
(VAPID/Ed25519; precisa decidir a dependência de criptografia — o daemon já traz `rcgen`/`rustls`, a verificar se cobrem P-256 bruto);
G3 = job-modelo por motor + redeploy; G4 = `generate:` no manifesto. O `shared` sobe de versão em G1 (comandos novos).

**Perguntas para o dono** (com a minha recomendação):
1. A GUI deve poder **mostrar o valor uma vez** logo após gerar? *Recomendo não*: o 👁 já cobre, com auditoria; mostrar na criação
   convida a copiar para um bloco de notas.
2. O agente pode pedir `kind: Symbols`? *Recomendo sim*, mas só com um aviso na resposta, porque é raro e quebra URL.
3. `NAME_PREVIOUS` deve **expirar sozinho** (ex.: 7 dias)? *Recomendo sim*: um segredo antigo vazado e esquecido é um risco; o
   padrão seria apagar após a rotação confirmada ou em 7 dias, o que vier primeiro.
4. `Rotate` deve **recusar** se algum serviço em execução ainda usa o valor (e avisar para fazer o redeploy)? *Recomendo avisar, não
   recusar*: o daemon sabe quais envs apontam para o secret e quais serviços estão rodando.

## 3. O que falta decidir (por causa do escopo maior da rotação)

- **Quem orquestra** os passos 2–4: um comando único no daemon (uma "rotação" com estado: gerou, trocou, atualizou, redeployou) ou
  uma receita montada pelo assistente/pela GUI?
- **Falha no meio** (passo 5): desfazer sozinho, parar e pedir ajuda, ou deixar as duas senhas válidas por um tempo?
- **Sem downtime**: bancos que aceitam duas senhas ao mesmo tempo (usuário duplicado) versus troca seca.
- **Motores fora da lista** (um banco do usuário que não é Postgres/RabbitMQ/Redis/RustFS/MinIO): como o rustploy sabe aplicar a troca?
- **Segredos que não são senha de serviço** (`JWT_SECRET`): trocar invalida sessões; é rotação "de aplicação", sem passo 2.


## 4. Próximo passo

Fechar as perguntas da seção 2 (1 a 4) e as da seção 3; só então escrever as fases definitivas.
