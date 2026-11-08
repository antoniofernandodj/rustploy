# Exportar e importar os dados de um serviço

> **Plano, nada implementado** (2026-11-08). Escrito para o dono revisar **antes** de qualquer código. As
> escolhas que precisam da sua resposta estão na seção 11. Origem: item "Exportar e importar DADOS de um
> container ou volume" do `TODO.md`, com a restrição que você deu: **o Rustploy não lida com containers; lida
> com serviços**.

## 1. O problema, com um exemplo

Você tem um **Postgres** rodando como serviço no Chiquitos, com dados de verdade. Quer:

- subir **uma cópia em outro servidor** (para testar uma migração com dados reais, ou mudar de VPS);
- ou **guardar os dados num arquivo** hoje e recuperá-los depois.

Hoje o export do Rustploy leva só a **configuração** (o YAML do serviço). Os dados ficam de fora. O que existe
é a *migração* de banco (`features/databases/migration.rs`), que copia de um banco para outro **dentro do mesmo
servidor**, passando por um dump em memória. Não há como tirar os dados do servidor.

## 2. A regra de design: tudo é uma operação **de um serviço**

O Rustploy mexe em containers **através dos serviços**. Então a exportação e a importação:

- recebem um **`service_id`** (nunca um id de container, nem um nome de volume solto);
- **descobrem sozinhas** quais volumes e qual banco pertencem àquele serviço;
- respeitam as regras que já existem para serviços (projeto, níveis de acesso, eventos, auditoria).

Exemplo do que **não** se faz: `ExportVolume { name: "rp_01abc_pgdata" }`. O nome do volume é um detalhe
interno que o usuário não deveria precisar conhecer, e dá a quem chama acesso a um volume de **outro** serviço.
O certo é `ServiceDataExport { service_id: "svc_01…" }`: o daemon sabe que aquele serviço tem o volume `pgdata`.

## 3. O que são "os dados" de um serviço

Depende do tipo do serviço. O daemon decide, o usuário só escolhe o serviço:

| Serviço | O que entra no pacote | Como |
|---|---|---|
| Banco (`product`: postgres, mysql, mariadb, mongodb) | um **dump lógico** do banco | o mesmo `pg_dump`/`mysqldump`/`mariadb-dump`/`mongodump` da migração, mas gravando num arquivo |
| Redis | o `dump.rdb` | `redis-cli --rdb` |
| Com volumes (Compose, ou `volumes` no serviço) | um **tar de cada volume** | num container descartável que monta o volume só para ler |
| Kafka, RabbitMQ, NATS e outros | o tar dos volumes | igual ao anterior (v1: sem dump lógico) |
| Imagem sem nenhum volume | nada | o plano avisa "este serviço não tem dados" |

**Por que dump lógico para banco e tar para o resto:** um tar do diretório de um Postgres rodando pode sair
**corrompido** (arquivos escritos no meio da cópia) e só serve para a **mesma versão** do motor. O dump é
consistente com o banco no ar e portável entre versões. Para um volume comum (uploads, por exemplo), o tar com
o serviço parado é o certo.

**Quais volumes são "do serviço":** os volumes nomeados da stack Compose (`<compose_project>_<volume>`, o nome
gravado do serviço) e os `volumes` declarados no spec. **Não** exporta pastas arbitrárias do servidor: só o que
o próprio serviço já declara montar.

## 4. Os comandos (no molde do que já existe)

Mesmo estilo de `ServiceExportPlan`/`ServiceExport` e do upload de zip (`/api/services/<id>/archive`): comandos
pequenos para controlar, rotas HTTP binárias para os bytes.

1. **`ServiceDataPlan { service_id }`** — só olha e responde: quais partes existem, o tamanho de cada uma, se o
   serviço precisa parar, se há espaço em disco. É o que a tela mostra antes de o usuário confirmar. Não muda
   nada.
2. **`ServiceDataExport { service_id, stop_service }`** — começa a exportação como uma **tarefa** (como a
   migração) e devolve um `export_id`. Os eventos de progresso chegam pelo SSE de sempre.
3. **`GET /api/services/<id>/data/export/<export_id>`** — baixa o pacote (stream, sem carregar tudo em memória).
4. **`POST /api/services/<id>/data/import`** — envia o pacote (corpo binário, como o zip). Antes, o daemon
   confere a integridade e **responde com o plano** do que vai ser substituído; só roda com a confirmação.
5. **`ServiceDataImport { service_id, confirm }`** — executa depois do envio, com `confirm: true`.

## 5. O pacote (`.rpdata`)

Um **tar** (aberto de propósito: dá para abrir à mão se algo der errado), com:

```
manifest.json          formato, versão do Rustploy, nome e tipo do serviço, motor e versão (se banco),
                       lista de partes com tamanho e SHA-256, data
db/<nome>.dump         o dump lógico (se banco)
volumes/<volume>.tar   um tar por volume
```

O `manifest.json` vem **primeiro**, para a importação conferir tudo antes de tocar em algo. Cada parte é
verificada pelo SHA-256 antes de ser aplicada.

## 6. Consistência: o que acontece com o serviço durante a exportação

| Tipo | Serviço no ar? | Observação |
|---|---|---|
| Banco (dump lógico) | **pode continuar no ar** | o dump é consistente por construção |
| Volume comum | **parado durante o tar** | o plano mostra "vai parar por ~N s". `stop_service: false` copia no ar, com aviso de que pode sair inconsistente |

Depois da cópia, o serviço volta ao estado em que estava (se estava rodando, volta a rodar).

## 7. Importar com segurança (o lado perigoso)

Importar **substitui** dados. Por isso:

- o serviço de destino **tem de existir** e ser do **mesmo tipo** (mesmo motor, e versão igual ou mais nova);
  criar o serviço continua sendo `ServiceImport`/wizard (a configuração), e **depois** vem o `ServiceDataImport`;
- o serviço é **parado** durante a importação, e a tela diz exatamente o que será substituído;
- só roda com `confirm: true`;
- **rede de segurança:** antes de substituir um volume, o daemon guarda uma **cópia do volume atual** (renomeia ou
  tira um tar para a pasta de dados) e só a apaga quando a importação termina bem; se falhar no meio, devolve o
  volume antigo. Nunca deixa o destino "pela metade";
- no banco, a restauração usa as credenciais do **próprio serviço** (o daemon as resolve; o valor nunca passa
  por quem chama), e no fim **compara as contagens por tabela** (a verificação que a migração já faz).

## 8. Exemplo completo: levar o Postgres do Chiquitos para outro servidor

1. **No servidor A** (origem), na tela do serviço `db`: **Exportar dados**. A tela mostra o plano: "Postgres,
   banco `chickie`, ~380 MB, o serviço continua no ar". Você confirma e baixa o `db.rpdata`.
2. **No servidor B** (destino): importe a **configuração** do serviço (`ServiceExport` do A → `ServiceImport` no
   B) e deixe-o criado.
3. **No servidor B**, na tela do serviço `db`: **Importar dados**, escolha o `db.rpdata`. A tela confirma o tipo
   e a versão e mostra "vai substituir o banco `chickie`". Você confirma.
4. O daemon restaura, compara as contagens por tabela com o `manifest.json` e mostra "341 tabelas conferidas".

Quando isso existir, o teste de migração Docker ↔ Podman do `TODO.md` pode usar dados reais exportados da VPS.

## 9. Segurança e o MCP

Um pacote de dados contém **tudo o que um agente nunca pode ver** (senhas de usuários, dados dos clientes). Por
isso:

- os comandos novos são **`admin`** no `required_level` (`api/agent/policy.rs`, que obriga a escolher um nível);
- e ficam **proibidos à chave de visitante** em qualquer nível (como `SecretReveal`): só a chave mestra exporta
  ou importa. O agente pode chamar o **`ServiceDataPlan`** (só metadados) no nível observador;
- o pacote **não é cifrado por padrão** na v1 (ver a pergunta 1): trate o arquivo como um backup do banco.

## 10. Como testar

- **Unidade:** o `manifest.json` (ida e volta, versão de formato), a verificação do SHA-256, a escolha das
  partes por tipo de serviço, a recusa de tipo/motor diferente, a recusa sem `confirm`. Com um engine falso, sem
  Docker.
- **Integração (marcada `#[ignore]`, precisa de Docker ou Podman, como os testes do alias de rede):**
  1. Postgres com dados → exporta → cria outro serviço → importa → contagens iguais;
  2. serviço com um volume cheio de arquivos → exporta → importa em outro → `diff -r` igual;
  3. falha no meio da importação → o volume antigo **volta** e o serviço sobe como antes;
  4. exportar com o serviço no ar × parado, conferindo o que o plano anunciou.
- **Na máquina de verdade:** exportar um banco do Chiquitos e importar num servidor de teste, antes de dar o
  item por fechado (a regra do projeto: nada de UI ou dado sem rodar de verdade).

## 11. Decisões que preciso de você

1. **Cifrar o pacote?** Sugestão: oferecer cifra com `age` (a chave de recuperação do `state_git` já existe) e
   deixar **ligada por padrão quando ela estiver configurada**. Sem ela, o arquivo sai em claro.
2. **Bind mounts do spec** (`volumes` com caminho do servidor): entram ou só os volumes nomeados? Sugestão: só os
   que o serviço declara, e **nunca** os caminhos sensíveis do próprio Rustploy (`/etc/rustploy`, `/var/lib/rustploy`).
3. **Tamanho:** há um teto (como os 100 MiB do zip) ou é ilimitado com stream? Sugestão: sem teto, com
   verificação de espaço em disco no plano.
4. **Importar em motor ou versão diferente:** recusar sempre (sugestão), ou permitir um dump de versão mais antiga
   para uma mais nova?
5. **Servidor → servidor direto** (o B puxa do A sem passar pelo seu computador): fora da v1? Sugestão: sim, fora.
6. **Onde fica na tela:** uma aba nova "Dados" no serviço, ou botões na aba Avançado? Sugestão: aba **Dados**,
   porque tem plano, progresso e histórico.

## 12. Fases sugeridas

1. `ServiceDataPlan` e exportar/importar **volumes** (tar) com a rede de segurança. Testes de unidade e de
   integração.
2. **Bancos** (Postgres, MySQL/MariaDB, MongoDB) com dump lógico e a conferência de contagens; Redis (RDB).
3. GUI e webui (aba **Dados**), com progresso e confirmação; docs pt e en.
4. Cifra com `age`.

**Reaproveitamento:** o mesmo mecanismo é a base do item "Backup de volumes para S3" do `TODO.md`: lá, em vez de
baixar o pacote, o daemon o envia a um destino S3 num agendamento. Vale desenhar o gerador do pacote como um
`Stream` que serve aos dois usos.

## 13. O que este plano não cobre

- Mover um serviço **inteiro** entre servidores num clique (configuração + dados): fica como um fluxo de duas
  etapas (seção 8), não como um comando único.
- Backup agendado (é o item do S3).
- Dados que **não** pertencem a nenhum serviço (volumes soltos, imagens): ficam de fora **de propósito**.
