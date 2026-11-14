# Plano: ajustes da recuperação de servidor (achados no teste de 2026-11-13)

> **Estado (2026-11-13): proposta.** A parte "Já corrigido" foi publicada junto da release das portas TCP.
> O resto espera aprovação do dono. Continuação de `docs/plano-backup-de-dados-para-s3.md` (Fase 4, a tela
> Recuperar).

## De onde vem

Em 2026-11-13 o dono destruiu o rustploy local (com o `uninstall.sh` novo) e recuperou tudo de novo,
várias vezes, até a recuperação trazer a infra do git e os dados do S3 de todos os serviços. Cada rodada
mostrou um defeito. Este documento junta os que ainda faltam, com o porquê de cada um, para não depender de
alguém lembrar.

O que o teste provou que funciona: com a chave certa, num servidor limpo, a recuperação traz projetos,
serviços, variáveis e segredos do git, restaura o banco (dump) e os volumes (s3, broker, cache) do S3, e
sobe tudo na ordem das dependências.

## Já corrigido (no código, sem commit)

| O que acontecia | O que mudou |
|---|---|
| O motivo de cada serviço que falhava só existia na tela; o log não dizia nada | o journal registra cada etapa e cada erro, por serviço |
| O backup automático disparava no primeiro minuto da recuperação e podia gravar o serviço **ainda vazio** como o backup mais recente | o agendador fica parado enquanto a recuperação está em curso |
| O sync com o git apontava para uma conta Git que a própria restauração tinha apagado, e parava | a conta é recriada e o sync reapontado |
| Com a chave errada para os **backups**, a restauração dos dados começava e os serviços falhavam no meio | o resumo confere a chave contra o backup de cada serviço e recusa começar; dá para informar outra chave sem reinstalar |
| Depois de uma recuperação, `s3`, `broker` e `cache` diziam "não tem volumes" e nunca mais faziam backup | os volumes são achados pelo que o container monta e pela etiqueta com o ID do serviço, nunca por nome |
| RabbitMQ criado pelo assistente subia sem `hostname` fixo, e cada container recriado abria uma pasta vazia, ignorando filas e mensagens | o assistente põe `hostname` fixo |
| O aviso da troca de chave dizia que "commits antigos" só abrem com a chave antiga (os segredos atuais são recifrados) | o texto diz o que é verdade: os backups no S3 e o histórico do git não mudam |

## O que falta

### 1. Contas Git depois da recuperação

**O que acontece.** Ao final do passo 1, o resumo mostra em vermelho: "reconecte Antonio em Settings → Git
providers… os clones de repositório privado vão falhar". Quem chega nessa tela acabou de digitar o token, a
URL e a chave, então a mensagem assusta e parece mandar refazer tudo.

Ela tem três problemas:

- **Avisa à toa.** Reclama de toda conta que voltou do git sem token (os tokens de login nunca vão para o
  git), mesmo que **nada** use essa conta. No teste, nenhum serviço nem job usava a "Antonio".
- **Deixa passar o problema real.** Os dois jobs que clonam o repositório `chickie` apontavam para uma conta
  (`gp_01M6GZF0…`) que não existia mais em lugar nenhum. Eles vão falhar ao clonar, e a mensagem não os cita.
- **Duplica contas.** Cada recuperação cria mais uma conta "Recuperação" com o token digitado. Depois de
  algumas tentativas o repositório tem várias, e a conta antiga continua lá sem token.

**O que fazer.**

1. **Avisar pelo que quebra, não pela conta.** Percorrer serviços (`git_source`) e jobs e listar:
   - os que usam uma conta **sem token** ("o job `migrations` usa a conta Antonio, que voltou sem token");
   - os que usam uma conta que **não existe** ("o job `migrations` aponta para uma conta Git apagada").
   Conta que ninguém usa não gera aviso. Se nada quebra, nenhuma mensagem.
2. **Aviso em amarelo, com a ação**: "Antes de usar estes jobs, escolha uma conta com acesso ao repositório
   em Settings → Git". Não é erro: nada da recuperação falhou.
3. **Reaproveitar em vez de criar.** Se a restauração trouxe uma conta do **mesmo** provedor (mesmo GitHub ou
   mesmo Gitea) sem token, pôr nela o token digitado, em vez de criar uma "Recuperação" nova. Assim o que já
   apontava para a conta antiga volta a funcionar sozinho.
   - **Cuidado:** o token digitado pode ser um PAT de escopo fino, com acesso só ao repositório da infra. Pôr
     esse token na conta antiga não dá acesso aos outros repositórios. Por isso o item 1 continua necessário:
     depois de pôr o token, testar o acesso a cada repositório que algo usa (um `ls-remote`) e avisar os que
     o token não alcança.
4. **Não duplicar.** Se já existe uma conta "Recuperação" do mesmo provedor, atualizar o token dela.

### 2. Uma tentativa com a chave errada inutiliza o servidor

**O que acontece.** Se a chave colada não abre os segredos do repositório, o daemon recusa o commit (certo),
mas logo depois grava um commit **próprio** no repositório local. A partir daí o servidor não é mais "limpo":
toda tentativa seguinte falha, **inclusive com a chave certa**, e a única saída é desinstalar e instalar de
novo. No teste, o dono tentou quatro chaves e nenhuma funcionou, porque a primeira tentativa errada já tinha
estragado o servidor.

É o engano mais provável num desastre de verdade: pressa e várias chaves na mão.

**O que fazer.** Se a restauração da infra falhar durante a recuperação, voltar o repositório local ao estado
de antes (sem commit nenhum) e desligar o sync que a recuperação ligou. O servidor continua limpo, e a próxima
tentativa funciona.

### 3. A mensagem de erro engana

**O que acontece.** Com a chave errada, a tela diz "o repositório não trouxe nenhum projeto (vazio, branch
errada ou outro repositório)". O motivo real (a chave não abre o segredo `ASAAS_API_KEY`) só aparece no
journal.

**O que fazer.** Levar o motivo da recusa até a tela: "a chave de recuperação informada não abre os segredos
deste repositório (o primeiro que falhou: `ASAAS_API_KEY`). Confira se é a chave deste servidor." E, quando o
servidor já não estiver limpo (item 2), dizer isso, em vez de culpar o repositório.

### 4. Conferir a chave antes de tocar em qualquer coisa

**O que acontece.** A checagem prévia de hoje confere a chave contra os **backups**, no fim do passo 1. A recusa
pela chave errada acontece **antes**, ao aplicar os segredos da infra.

**O que fazer.** No começo do passo 1, antes de aplicar qualquer coisa: buscar o repositório, tentar abrir um
segredo com a chave colada e, se não abrir, parar ali, com a mensagem do item 3. Com isso os itens 2 e 3
passam a ser rede de segurança, não o caminho normal.

### 5. Backup automático de um serviço cuja restauração falhou

**O que acontece.** A trava do agendador vale só enquanto a recuperação está em curso. Quando ela termina com
algum serviço falho, o agendador volta e pode gravar esse serviço **vazio** como o backup mais recente. No
teste isso aconteceu: 25 segundos depois de uma recuperação desistir, o `db` vazio virou o backup mais novo.
Com retenção de 7, uma semana de backups vazios empurra os bons para fora.

**O que fazer.** Segurar o backup automático de cada serviço cuja restauração falhou, com um aviso na aba Dados
dele, até alguém agir.

**Decisão do dono:** o que libera o serviço. Opções: um botão "liberar backups" na aba Dados; ou liberar
sozinho quando uma restauração manual daquele serviço der certo; ou as duas coisas.

### 6. RabbitMQ antigo sem `hostname`

**O que acontece.** O assistente já foi corrigido, mas RabbitMQ criados antes (em qualquer servidor) continuam
sem `hostname` fixo. Numa recuperação, os dados deles voltam para o volume e o RabbitMQ não os enxerga.

**O que fazer.** No resumo da recuperação, e na aba Dados do serviço, avisar quando um serviço RabbitMQ não tem
`hostname` no compose: "os dados deste serviço não vão aparecer depois de recriar o container".

## Ordem sugerida

1. Itens 2, 3 e 4 juntos: são o mesmo caminho (a chave errada) e o que mais assusta numa emergência.
2. Item 1 (contas Git).
3. Item 5, depois da decisão.
4. Item 6.

## Como testar

O mesmo roteiro de 2026-11-13: dados de teste em cada serviço, backup, `uninstall.sh`, instalar, Recuperar.
Mais quatro casos:

- colar uma chave **errada** primeiro e a certa depois, sem reinstalar: tem de funcionar na segunda;
- um job que clona repositório privado: o resumo tem de citá-lo se a conta dele não tiver token;
- recuperar duas vezes seguidas: não pode sobrar mais de uma conta "Recuperação";
- um serviço cuja restauração falhou: o backup automático dele não pode rodar.
