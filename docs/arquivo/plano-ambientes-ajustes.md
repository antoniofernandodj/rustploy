# Ambientes: explicar os botões, comparar por nomes, trazer o que está no ar e destruir

> **Arquivado em 2026-11-25** (revisão das docs): concluído ou substituído. O que ainda vale fazer foi para o
> `TODO.md`; o que não fazia mais sentido saiu daqui.

**Status:** implementado na v0.1.98 (daemon 0.1.82, app 0.1.89), em 2026-11-24.

Continuação do [plano de Release/Ambiente/Rota/Volume](plano-releases-ambientes-rotas-volumes.md) §2.
O dono criou três ambientes do Chiquitos (staging, dev, prod) e, na aba Ambientes, não viu botão
nenhum, nem entendeu por quê. Conferido pelo MCP (só leitura) no Chiquitos em 2026-11-24.

## O que estava errado

1. **"origem — · aqui — · imagens diferentes" sem botão.** Os serviços Git do Chiquitos estão no ar
   desde antes da v0.1.91, quando as releases começaram, e não tiveram deploy desde então: não há
   release na produção (`ReleaseList` do `worker` vazio). Sem release, não havia imagem para levar, e a
   tela ainda dizia "imagens diferentes" quando nenhum lado tinha imagem.
2. **Stacks Compose (broker, cache, db) sem botão e sem explicação.** Compose nunca é promovido (não há
   uma imagem só); a tela não dizia isso.
3. **Ruído na comparação.** `depends_on`, vínculos e pré-deploy apareciam como diferença, em JSON cru,
   porque a cópia aponta para os serviços e jobs **dela** (ids próprios). Uma cópia fiel parecia
   diferente da origem.
4. **Sem como apagar um ambiente de uma vez.** Era preciso apagar serviço por serviço (respeitando as
   dependências) e depois o projeto.
5. **(pedido junto)** O cartão do serviço na lista do projeto não tinha botão de deploy.

## O que mudou

### Uma nota por serviço na comparação

`EnvironmentServicePair.note` (campo novo, só na resposta): `compose`, `no_release_anywhere`,
`no_release_in_origin`, `no_release_here` ou `running_without_release`. As duas telas traduzem em uma
frase sob o nome do serviço ("Stack Compose: não há uma imagem só para levar. Faça o deploy em cada
ambiente."; "Este ambiente ainda não tem deploy: traga a versão da origem ou faça um deploy."…).
"Imagens diferentes" só aparece quando os dois lados têm release.

### Trazer/promover o que está no ar desde antes das releases

Um serviço sem release, mas com um deploy `Live` e o container rodando, passa a poder ser levado.
Ao levar (`EnvironmentPromote`), o daemon registra a imagem do container que está rodando como a
release daquele deploy (`ensure_baseline_release`) e segue como sempre. A comparação, que só lê, já
mostra o botão e a nota `running_without_release`.

**[decisão minha]** A release registrada guarda a configuração **de agora** do serviço: a do deploy
antigo não foi gravada em lugar nenhum. É o mais próximo do que está no ar; a imagem é exata.
**[decisão minha]** Só registra quando a pessoa leva (promover/trazer). Não há migração no boot que
crie releases para todos os serviços antigos: nada muda num servidor que só atualiza.

### Comparar por nomes

Antes de comparar, os ids de `depends_on`, vínculos e pré-deploy viram os nomes (do serviço ou do job).
A cópia fiel não mostra diferença; uma diferença de verdade aparece com nomes (`depends_on: [db] → [db,
cache]`). O que a cópia tira de propósito (dependência de banco compartilhado de outro projeto)
continua aparecendo, agora com o nome.

### Destruir ambiente

Comando `EnvironmentDestroy { project_id, confirm_name, delete_volumes }` (nível Admin, como apagar
projeto). Recusa um projeto que não é ambiente (a origem nunca é destruída assim) e um nome de
confirmação errado. Apaga os serviços **na ordem das dependências** (quem depende primeiro; o daemon
recusa apagar um serviço do qual outro depende), os jobs, as rotas e o projeto.
`delete_volumes` (padrão `false`) apaga os volumes próprios dos serviços; sem ele os dados ficam como
órfãos na tela Volumes. Volumes emprestados (`external`) nunca saem por aqui.

Na tela, nos dois clientes: botão **Destruir ambiente** no topo da aba Ambientes de um ambiente e
**Destruir** em cada linha da lista de ambientes da origem (pedido do dono). Os dois abrem o mesmo
painel: o que vai acontecer, um campo onde se digita o nome do ambiente, a caixa "Apagar também os
dados" (desmarcada) e Cancelar / Destruir ambiente. Destruído de dentro dele, a tela volta para a
origem.

### Deploy no cartão do serviço

Botão ▶ (verde) no cartão de cada serviço na lista do projeto, nos dois clientes. É o mesmo deploy da
tela do serviço, inclusive a pergunta das dependências.

## Atualizar um servidor em produção

Nada que já existe muda: nenhum formato (o YAML de infra e o SQLite são os mesmos), nenhum nome. A
comparação só lê. A release a partir do container só é gravada quando alguém clica em levar, e é uma
linha nova na tabela `release` (que a versão anterior lê normalmente). O comando novo só age quando
chamado. Teste de atualização com dados: **não se aplica** pela regra da skill (não mexe em proxy,
volumes, rede ou nomes); o roteiro abaixo roda num daemon que simula o estado antigo (release apagada
de um serviço no ar).

**Voltar de versão:** a v0.1.97 lê o mesmo estado. Releases registradas a partir do container aparecem
nela como releases normais.

## O que foi verificado

- Teste unitário: a cópia fiel não tem diferença de `depends_on`/vínculos/pré-deploy por nome.
- Ponta a ponta num daemon isolado com Podman (`e2e_envs.py`): origem com um serviço zip que depende de
  uma stack Compose e tem vínculo com ela, release apagada no SQLite (como na produção antiga) →
  criar ambiente → nota `running_without_release` e "Trazer" disponível; Compose com nota `compose` e
  sem botões; sem ruído de ids → Trazer da origem: deploy Live, a origem ganha a v1 a partir do
  container e a release do staging diz "promovida de … v1" → mesma imagem, sem botões → destruir com o
  nome errado e destruir a origem são recusados → destruir: o ambiente some, o container dele sai, a
  origem continua no ar. A primeira rodada achou um defeito real (apagar o `db` antes do `web`, que
  depende dele, era recusado) e a ordem das dependências entrou.
- GUI no Xvfb: notas, botões Destruir nos dois lugares, o painel, o ▶ disparando o deploy (com a
  pergunta das dependências). Webui no Chrome headless: as mesmas telas e o fluxo completo de destruir
  (nome errado recusado, nome certo destrói e volta para a origem).
- **Não verificado:** digitar o nome no painel da GUI (o Xvfb não entrega teclado; o mesmo fluxo foi
  provado pela webui e pela API).
