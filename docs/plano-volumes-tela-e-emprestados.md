# Volumes: tela própria, "usar volume existente" e volumes emprestados

**Status:** implementado na v0.1.96 (daemon 0.1.80, app 0.1.87), em 2026-11-24.

Continuação do [plano de Release/Ambiente/Rota/Volume](plano-releases-ambientes-rotas-volumes.md).
Lá, o volume ganhou dono (o serviço que o monta), passou a ser listado no Runtime e a ficar como
"órfão" quando o serviço é apagado sem apagar os dados. Faltavam três coisas.

## O problema

1. **Não havia um lugar para ver os dados.** Os volumes apareciam no meio da tela Runtime, junto
   com imagens e redes. Para responder "o que guarda os dados do banco? quem monta? tem backup? o
   que sobrou de serviços apagados?", era preciso cruzar telas.
2. **Uma stack Compose não tinha como reaproveitar um volume que já existe** (o banco de um serviço
   apagado, um volume criado à mão) sem a pessoa saber escrever o bloco `external` do Compose.
3. **Um defeito com dados:** um volume declarado como `external: true` num Compose foi criado por
   outra pessoa ou outra stack, e a stack só o usa. Mesmo assim, ao apagar o serviço marcando
   "Apagar os dados", o daemon apagava esse volume junto. Reproduzido na 0.1.79 (v0.1.95) pelo teste
   de atualização (ver abaixo).

## O modelo

- **Volume próprio**: a stack o criou (declarado sem `external`). Dono = o serviço. "Apagar os dados"
  apaga.
- **Volume emprestado**: declarado com `external: true` (ou o formato antigo `external: {name: x}`).
  Já existia antes da stack; ela só o monta. Dono, para fins de listagem, também é o serviço que o
  monta (assim ele não aparece como "solto"), mas **apagar o serviço nunca leva o emprestado junto
  sem uma pergunta própria**. Se ninguém mais o usa depois, ele vira órfão com a origem anotada, como
  qualquer outro.

O que o código lê do arquivo Compose fica em
`rustploy-daemon/crates/daemon/src/features/service_data/compose_volumes.rs`: quais volumes nomeados
cada serviço da stack monta, onde, e quais são externos. Pasta do servidor (`./x`, `/srv/x`) não é
volume e é ignorada. Arquivo que não é YAML válido devolve "não sei" (nunca erro: quem valida o
arquivo é o deploy).

## Protocolo (só cresceu)

- `ServiceDelete` ganhou `delete_external_volumes` (padrão `false`). `delete_volumes` passou a valer
  só para os volumes próprios.
- `VolumeList` (Observer): cada volume com tamanho, em uso, dono, órfão, política de backup,
  anônimo e **quem monta** (`mounts`: serviço, projeto, caminho no container e se é emprestado).
- `ComposeInsertExternalVolume { content, volume_name }` (Observer, não grava nada): devolve o texto
  do Compose com o bloco `external` inserido e a **chave** a usar na montagem. A inserção é textual:
  comentários e ordem do arquivo ficam; entra no `volumes:` do topo se houver, senão cria o bloco.
  Se o volume já estiver declarado como externo, devolve o arquivo igual. Chave ocupada ganha sufixo.
- `DockerVolumes` passou a mostrar o dono também do volume emprestado.

## As telas (GUI e webui)

- **Barra lateral → Volumes**: um cartão por volume com nome, tamanho, estado (em uso / sem uso /
  órfão com a origem), selo "backup ligado" e quem monta cada um ("db (loja) → /var/lib/postgresql",
  com "· emprestado" quando for o caso). Filtros "Só órfãos" e "Mostrar anônimos" (anônimos ficam
  escondidos por padrão: são os de 64 caracteres hexadecimais que o motor cria sozinho).
  - **Adotar num serviço**: só para volume que ninguém usa. Escolhe um serviço Application e o
    caminho no container; vira um `ServiceVolumeAttach`, que vale no próximo deploy. Decisão
    conservadora: não ofereço adotar um volume em uso (dois containers nos mesmos dados).
  - **Apagar**: só para volume sem uso, com confirmação que diz que é sem volta.
- **Editor do Compose (aba Geral do serviço) → "Usar volume existente"**: abre uma lista dos volumes
  nomeados sem uso (os órfãos primeiro, com a origem). "Inserir" chama o
  `ComposeInsertExternalVolume` e troca o texto do editor; nada é salvo até a pessoa clicar em
  "Salvar compose". A montagem no serviço (`- chave:/caminho`) a pessoa escreve: o Rustploy não
  adivinha em qual serviço da stack nem em qual caminho.
- **Apagar um serviço**: depois das duas perguntas que já existiam (apagar? e os dados?), uma
  **terceira**, só quando a stack monta volumes emprestados, listando-os pelo nome: "Apagar os
  emprestados" ou "Manter os emprestados". Padrão: manter.
- **Ingress → Nova rota / Editar rota**: botão **Cancelar** ao lado de "Salvar" na GUI, que fecha o
  editor sem gravar e volta à lista do Ingress (pedido do dono junto com esta entrega; a webui já
  tinha).

## Atualizar um servidor em produção

O que muda para quem já tem dados: **só o "Apagar os dados" ao apagar um serviço**, que passa a apagar
menos (mantém os emprestados). Nada é renomeado, nenhum campo gravado muda de significado, nenhum
formato muda (o YAML de infra e o SQLite são os mesmos da 0.1.79: os comandos novos só leem).

Conferido pelo MCP, só leitura, em 2026-11-24:

- **Chiquitos** e **Itemize**: todas as stacks Compose usam volumes próprios; nenhuma declara
  `external`. Nada muda para elas.
- **Servidor dev do Itemize (Flow)**: a stack `db` declara `flow_mysql_data` como `external`. Na
  0.1.79, apagar esse serviço com "Apagar os dados" apagaria o banco do Flow; na nova, só com a
  terceira pergunta respondida "Apagar os emprestados".

**Teste de atualização** (daemon isolado, Podman, mesma máquina):

1. daemon **0.1.79** publicado (binário do `.deb` da v0.1.95), num `db_path` novo: projeto com secret,
   stack Compose com volume próprio e dado gravado, stack Compose com um volume externo (como o do
   Flow) com dado gravado, serviço Application com domínio. Na mesma fase, o defeito do item 3 se
   reproduz: "apagar os dados" de uma stack apagou o volume emprestado dela.
2. daemon parado; o binário **novo** sobe no mesmo `db_path`.
3. Conferido: os três serviços continuam lá; o dado do banco e o do volume emprestado estão intactos
   e montados; o domínio continua roteado; redeploy do banco (mesmos dados) e do Application
   funcionam; o `VolumeList` mostra o emprestado com dono e marcado como emprestado.

Roteiro dos emprestados no daemon novo (`e2e_borrowed.py`): inserir o bloco pelo comando, montar,
deploy lendo o dado antigo, `VolumeList`/`DockerVolumes` com dono, apagar com `delete_volumes` (o
próprio sai, o emprestado fica e vira órfão com a origem), reusar o órfão noutra stack (tem dono de
novo) e apagar com `delete_external_volumes` (aí ele sai).

**Voltar de versão:** a 0.1.79 lê o mesmo estado sem perder nada (não há campo novo gravado). Só volta
a ter o defeito do item 3.

## O que ficou para depois

- Montar automaticamente a chave inserida num serviço da stack (exigiria escolher serviço e caminho
  na tela; hoje a pessoa escreve uma linha).
- Mostrar na tela Volumes o último backup de cada volume (hoje só se há política ligada).
- Tamanho dos volumes no Podman: o `df` dele não informa o tamanho de alguns volumes (aparece "—"
  ou "0 B"); o dado vem do motor.
