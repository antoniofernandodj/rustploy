# Rotação de segredos (senha de banco, broker, storage, token)

> **Estado: SÓ PROJETADO, não implementado.** Nasceu em 2026-11-04 ao tirar a rotação da Fase G de
> `docs/plano-credenciais-cifradas-e-storage.md`, porque rotacionar é mais amplo que gerar. Depende da
> **G1** (`SecretGenerate`) desse plano. Este documento junta o que já tinha sido pensado e as perguntas que
> ainda precisam de você; ele ainda não é um plano fechado.

## 1. O que "rotacionar a senha do banco" realmente é

Gerar um valor novo é só o primeiro passo. O ato inteiro é:

1. **gerar** a senha nova (G1);
2. **trocá-la no sistema de fora** — o Postgres, o RabbitMQ, o Redis, o RustFS/MinIO — com um comando específico de cada motor;
3. **atualizar o rustploy**: o secret, e as variáveis/URLs que apontam para ele (`DATABASE_URL` etc.);
4. **redeployar** os serviços que usam o valor (o `ServiceReload` só reinicia o mesmo container e **não** relê env/secrets);
5. **tratar a falha no meio**: se o passo 2 deu certo e o 4 não, o banco já tem a senha nova e os apps ainda têm a velha.

Hoje isso é feito à mão. No Chiquitos (2026-11-03) foi: job com `ALTER USER`, troca do secret, redeploy, e sobrou
um `POSTGRES_PASSWORD_OLD` que ainda precisa ser apagado.

## 2. O que já estava desenhado (vindo da Fase G)

- `SecretRotate { project_id, service_id?, name, kind, length? }`: numa transação, o valor atual vira `NAME_PREVIOUS`
  e um novo assume `NAME`. As variáveis que apontam para `NAME` não mudam. Entrega ao job de troca **as duas senhas**
  (`PGPASSWORD=${NAME_PREVIOUS}` e `NEWPW=${NAME}`). Depois de confirmar, `SecretDelete` do `_PREVIOUS`.
- **Job-modelo por motor**, criado e executado pelo botão "Rotacionar": Postgres (`ALTER USER`), RabbitMQ
  (`rabbitmqctl change_password`), Redis, RustFS/MinIO. Termina com o redeploy dos serviços que usam o secret.
- Botão **"Rotacionar"** na linha do secret (GUI e webui) e um comando equivalente para o agente.
- O agente nunca vê nenhum dos dois valores.

## 3. Perguntas já levantadas (com a recomendação de então)

1. `NAME_PREVIOUS` deve **expirar sozinho** (apagar após a rotação confirmada ou em 7 dias)? *Recomendo sim.*
2. `Rotate` deve **recusar** se algum serviço em execução ainda usa o valor, ou só avisar? *Recomendo avisar, não recusar:* o daemon
   sabe quais envs apontam para o secret e quais serviços rodam.

## 4. O que falta decidir (novo, por causa do escopo maior)

- **Quem orquestra** os passos 2–4: um comando único no daemon (uma "rotação" com estado: gerou, trocou, atualizou, redeployou) ou
  uma receita montada pelo assistente/pela GUI?
- **Falha no meio** (passo 5): desfazer sozinho, parar e pedir ajuda, ou deixar as duas senhas válidas por um tempo?
- **Sem downtime**: bancos que aceitam duas senhas ao mesmo tempo (usuário duplicado) versus troca seca.
- **Motores fora da lista** (um banco do usuário que não é Postgres/RabbitMQ/Redis/RustFS/MinIO): como o rustploy sabe aplicar a troca?
- **Segredos que não são senha de serviço** (`JWT_SECRET`): trocar invalida sessões; é rotação "de aplicação", sem passo 2.

## 5. Próximo passo

Fechar as perguntas das seções 3 e 4 **depois** da G1. Quando fechadas, reescrever este documento como plano de fases.
