# As chaves do Rustploy, explicadas do zero (e repetidas de propósito)

> **Status:** documento de explicação. Não descreve uma feature nova: descreve o que **existe hoje** e, no
> fim, o que foi **proposto** (o assistente de chave de recuperação), claramente separado.
> **Idioma:** pt-BR, de propósito repetitivo. Se algo parecer dito duas, três ou dez vezes, é de propósito.
> **Por que existe:** numa conversa, as explicações misturaram duas chaves diferentes e se contradisseram.
> Este documento é a versão arrumada. A Parte 12 lista exatamente o que foi dito errado antes.

---

## Parte 0 — Se você só ler UMA parte, leia esta

1. O Rustploy guarda **segredos** (senhas, tokens, chaves de API). Ele guarda eles **embaralhados**
   (cifrados), nunca em texto puro. O formato embaralhado aparece como `ENC[age:…]`.
2. Para embaralhar e desembaralhar, ele usa **chaves**. Uma chave é como uma chave de porta.
3. Existem **DUAS chaves diferentes** no sistema. Não uma. **Duas.** E elas têm funções diferentes:
   - **Chave do servidor.** É a "chave da porta de casa". Fica **dentro** do servidor. O servidor usa ela
     no dia a dia. **É criada sozinha**, você não precisa fazer nada.
   - **Chave de recuperação.** É a "chave reserva guardada na casa de um amigo". Fica **fora** do servidor,
     com **você**. Só serve para o dia em que o servidor **morrer**. **Não é criada sozinha**: hoje
     você precisa criar (ou, se o assistente proposto for feito, o assistente cria).
4. Cada uma dessas chaves tem **duas metades**: uma **privada** (secreta) e uma **pública** (pode ser
   mostrada). Então são, no total, **quatro peças**: privada do servidor, pública do servidor, privada da
   recuperação, pública da recuperação.
5. Se você perder o servidor **e** não tiver a chave de recuperação privada guardada fora, **os segredos
   não voltam**. Nunca. Nem o desenvolvedor consegue recuperar. É matemática, não é um erro do sistema.

Se ficou claro, ótimo. Se não ficou, continue: o resto repete isso com calma, de vários ângulos.

---

## Parte 1 — O que é "cifrar" (sem jargão)

Imagine que você quer guardar a senha do banco de dados num papel, mas não quer que quem achar o papel
consiga ler. Você escreve a senha e coloca o papel dentro de um **cofre**. O cofre tem uma **fechadura**.
Só quem tem a **chave** abre.

- **Cifrar** = colocar a senha no cofre e trancar.
- **Decifrar** = abrir o cofre com a chave e ler a senha.
- **`ENC[age:…]`** = o cofre trancado. É um texto sem sentido (`ENC[age:YWdlLWVuY3J5cHRpb24ub3Jn…]`).
  Se alguém o ler, não entende nada.
- **`age`** é o **nome do programa/formato** de cofre usado. Não é uma sigla que você precise decorar. É só
  o nome da ferramenta. Quando você ver "age" no Rustploy, leia "o sistema de cofre que o Rustploy usa".

Repetindo, de outro jeito: um segredo no Rustploy **nunca** é gravado como `senha123`. É gravado como
`ENC[age:coisa-sem-sentido]`. Só quem tem a chave certa transforma de volta em `senha123`.

---

## Parte 2 — O que é um "par de chaves" (a parte que mais confunde)

Aqui está a ideia que, se você entender, resolve metade da confusão.

O sistema `age` não usa **uma** chave. Usa um **par**: duas chaves ligadas matematicamente.

| Metade | Como começa | Pode mostrar para os outros? | Para que serve |
|---|---|---|---|
| **Pública** | `age1…` | **Sim.** Pode colar em qualquer lugar | **Trancar** (cifrar) |
| **Privada** | `AGE-SECRET-KEY-1…` | **NÃO. Nunca.** | **Destrancar** (decifrar) |

Pense num **cadeado aberto** e na **chave dele**:

- A **chave pública** é como um **cadeado aberto** que você pode distribuir à vontade. Qualquer pessoa pode
  pegar um cadeado aberto e **fechar** (trancar) uma caixa com ele.
- A **chave privada** é a **chave do cadeado**. Só ela **abre**. Quem tem o cadeado aberto consegue
  trancar, mas **não** consegue abrir depois. Só quem tem a chave privada abre.

Consequência importante, para não esquecer:

- Com a **pública**, dá para **trancar** mas **não** dá para **abrir**.
- Com a **privada**, dá para **abrir**. (E a pública é derivada dela: da privada se tira a pública, mas da
  pública **não** se tira a privada.)

Por isso a pública pode ficar num arquivo de configuração à vista, e a privada é o que você protege.

**Resumo desta parte (repetido):** um par = uma pública (`age1…`, para trancar, pode mostrar) + uma privada
(`AGE-SECRET-KEY-1…`, para abrir, nunca mostrar). Guarde isso, vamos usar o tempo todo.

---

## Parte 3 — As DUAS chaves do Rustploy (o ponto central)

Agora juntamos: o Rustploy tem **dois pares**. Cada par tem uma pública e uma privada.

### Par A — a chave DO SERVIDOR

- **Função:** o servidor abrir os **próprios** segredos no dia a dia (por exemplo, ler a senha do banco para
  entregar ao container).
- **Onde fica a privada:** **dentro do servidor**, em **um destes dois lugares** (o servidor usa um deles,
  não precisa dos dois; a Parte 4 explica a ordem):
  - **Lugar 1 (o normal num servidor instalado pelo `.deb`): um arquivo.**
    - **Diretório:** `/etc/rustploy/`
    - **Arquivo:** `/etc/rustploy/age.key`
    - **Conteúdo do arquivo:** só **uma linha**, a própria privada: `AGE-SECRET-KEY-1…`. Mais nada.
    - **Permissão:** `0600` (só o dono lê e escreve). **Dono:** o usuário `rustploy`. O diretório
      `/etc/rustploy/` é `0750`.
    - **Como o `config.toml` sabe onde está:** na seção `[secrets]` há a linha
      `age_key_path = "/etc/rustploy/age.key"`. Isso é só o **caminho** do arquivo, **não** é a chave.
  - **Lugar 2 (alternativa, quando o daemon criou a chave sozinho): dentro do `config.toml`.**
    - **Diretório:** `/etc/rustploy/`
    - **Arquivo:** `/etc/rustploy/config.toml`
    - **Parte do arquivo:** a seção **`[secrets]`**, na linha `age_identity = "AGE-SECRET-KEY-1…"`
      (a privada inteira entre aspas, na mesma linha).
    - **Permissão do arquivo:** `0640`, dono `rustploy`.
  - **Exemplo do `config.toml` no Lugar 1** (a privada está no arquivo `age.key`, não aqui):
    ```toml
    [secrets]
    age_key_path = "/etc/rustploy/age.key"
    ```
  - **Exemplo do `config.toml` no Lugar 2** (a privada está aqui dentro):
    ```toml
    [secrets]
    age_identity = "AGE-SECRET-KEY-1QQQQ…"
    ```
  - **Atenção:** nunca ponha a privada em **outra** linha ou seção do `config.toml`. O daemon só procura
    `age_identity` **dentro de `[secrets]`**.
- **Onde fica a pública:** não precisa de arquivo. O servidor a **calcula** a partir da privada quando
  precisa. Ela aparece nos **logs** na hora em que a chave é criada.
- **Quem cria:** **sozinho.** O instalador, ou o próprio daemon na primeira vez que sobe. Você não faz nada.
- **Analogia:** a **chave da porta da sua casa**. Fica no seu bolso/na casa. Você usa todo dia.

### Par B — a chave DE RECUPERAÇÃO

- **Função:** abrir os segredos **se o servidor for perdido** (disco morreu, VPS apagada, `/etc/rustploy`
  apagado). É uma **chave reserva**.
- **Onde fica a privada:** **fora do servidor**. Com **você** (gerenciador de senhas, pendrive, papel num
  cofre). **Isto é essencial**: se a privada reserva morar no servidor, ela morre junto com o servidor e não
  adianta nada.- **Onde fica a pública:** num arquivo do servidor, a linha `recovery_recipient = "age1…"` em `config.toml`,
  dentro da seção `[secrets]`. Esta é a **única** linha da chave de recuperação que fica no servidor, e ela é
  a **pública** (a que só **tranca**).
- **Quem cria:** **hoje, ninguém sozinho.** Você precisa rodar `rustploy-cli keygen`, guardar a privada fora
  e colar a pública no `config.toml`. (O assistente proposto na Parte 11 faria isso por você.)
- **Analogia:** a **chave reserva que você deixa na casa de um amigo**. Ela **precisa** estar fora da sua
  casa. Se estiver dentro, o dia em que a casa pegar fogo você perde as duas.

### A tabela que resolve tudo (olhe ela várias vezes)

| | **Chave DO SERVIDOR** (Par A) | **Chave DE RECUPERAÇÃO** (Par B) |
|---|---|---|
| Para que serve | o servidor abrir seus segredos no dia a dia | abrir tudo se o servidor for perdido |
| Analogia | chave da porta de casa | chave reserva na casa de um amigo |
| Privada fica | **no servidor** | **fora do servidor, com você** |
| Pública fica | calculada a partir da privada (não tem arquivo) | `recovery_recipient` no `config.toml` |
| Criada por | **sozinha** (instalador/daemon) | **ninguém, hoje** (você, com `keygen`) |
| Você usa no dia a dia? | o servidor usa, você não | **não**, só no desastre |
| Se perder | o servidor ainda pode ter a recuperação | os segredos só voltam se a de recuperação existir |

**Repetição final da Parte 3:** são **duas** chaves. A do servidor está **dentro** dele e é automática. A de
recuperação tem a privada **fora** dele, com você, e hoje é manual.

---

## Parte 4 — Onde está a chave PRIVADA DO SERVIDOR (e por que viram "dois nomes")

Aqui nasceu parte da confusão. A **mesma** coisa (a privada do servidor) pode ficar em **dois lugares**
possíveis. São duas **formas de guardar a mesma coisa**, não duas chaves.

### Forma 1 — um arquivo: `/etc/rustploy/age.key`

- É um arquivo só com a privada: uma linha que começa com `AGE-SECRET-KEY-1…`.
- Permissão `0600`, dono `rustploy` (só o usuário do Rustploy lê).
- No `config.toml` existe a linha `age_key_path = "/etc/rustploy/age.key"` que **aponta** para esse arquivo.

### Forma 2 — dentro do `config.toml`: `age_identity = "AGE-SECRET-KEY-1…"`

- É a **mesma privada**, só que escrita **dentro** do `config.toml`, na seção `[secrets]`.
- "Identidade" é só o nome que o age dá à **chave privada** (`age_identity` = "a privada, dentro do config").

### A regra de ouro desta parte

> **`age.key` (arquivo) e `age_identity` (linha no config) são a MESMA COISA: a chave PRIVADA DO SERVIDOR.**
> O servidor usa **uma das duas**. Não precisa das duas.

Repetindo: `age.key` = privada do servidor **em arquivo**. `age_identity` = privada do servidor **dentro do
config.toml**. **São a mesma chave guardada em dois lugares possíveis.**

### Qual é usado? A ordem que o daemon segue (confirmado no código, `platform/secrets.rs`, função `open`)

Quando o daemon liga, ele procura a privada do servidor **nesta ordem**:

1. **Tem `age_identity` no `config.toml`?** Se sim, usa essa.
2. **Senão**, o arquivo apontado por `age_key_path` existe? Se sim, usa o arquivo.
3. **Senão (não achou nada):**
   - **Se já existem segredos `ENC[age:…]` gravados:** o daemon **se recusa a subir** e mostra um erro. Ele
     **não** cria chave nova, porque criar uma chave nova deixaria esses segredos **ilegíveis para sempre**.
     (É uma trava de segurança. Boa.)
   - **Se não existe nenhum segredo cifrado ainda:** ele **cria** uma chave nova e a grava no `config.toml`
     como `age_identity` (se não conseguir escrever o `config.toml`, grava no arquivo `age.key`).

### E o instalador (o pacote `.deb`)? (`packaging/debian/postinst`)

O instalador faz o seguinte, **antes** do daemon ligar pela primeira vez:

1. Garante a linha `age_key_path = "/etc/rustploy/age.key"` no `config.toml`.
2. Se o arquivo `age.key` não existe **e** não há `age_identity` **e** o programa `rustploy-cli` existe, roda
   `rustploy-cli keygen`, pega só a **privada** e grava em `/etc/rustploy/age.key` com permissão `0600`.

**Então, num servidor instalado pelo `.deb` normal, o `age.key` (arquivo) é criado pelo INSTALADOR.** O
caminho "o daemon cria e escreve no config.toml" só acontece quando o daemon sobe **sem** instalador (por
exemplo num teste, ou se o `rustploy-cli` não existia).

**Resumo da Parte 4 (repetido):** a privada do servidor é **uma coisa só**. Pode estar em `age.key` (caso do
`.deb` normal) ou em `age_identity` dentro do `config.toml` (caso do daemon rodando sem o instalador). O
servidor usa o que achar. Ela **nunca** sai do servidor, e **não** é a chave de recuperação.

---

## Parte 5 — Como o Rustploy usa as duas chaves JUNTAS (cifrar e decifrar)

Esta é a parte que explica **por que** existem duas.

### Cifrar (trancar um segredo)

Quando o Rustploy guarda um segredo, ele o tranca **para dois destinatários ao mesmo tempo**:

1. a **pública do servidor** (Par A), e
2. a **pública de recuperação** (Par B), **se ela estiver configurada**.

Imagine um cofre com **duas fechaduras**, e cada uma abre com uma chave **diferente**. O mesmo segredo trancado
dentro pode ser aberto por **qualquer uma** das duas chaves privadas: a do servidor **ou** a de recuperação.
(Confirmado: `encrypt` e `encrypt_file` fazem `recipients = [servidor] + [recuperação, se houver]`.)

### Decifrar (abrir um segredo)

- **No dia a dia:** o servidor abre com a **privada do servidor**. Funciona, e a recuperação nem é usada.
- **Só durante uma recuperação:** se a chave do servidor **não** abre (porque é um servidor novo, com chave
  nova, e os segredos foram trancados para o servidor **antigo**), o Rustploy tenta a **privada de
  recuperação**, que o usuário informou naquele momento e que fica **só na memória**.

### Por que isso é bom

Como o segredo foi trancado para as **duas**, você pode abrir com **qualquer** uma. Perdeu o servidor (e a
privada dele)? A de recuperação ainda abre. Foi isso que quis dizer "a recuperação é um seguro".

**Resumo da Parte 5:** o segredo é trancado para **duas** chaves públicas. Abre com **qualquer** uma das duas
privadas. A do servidor é a do dia a dia; a de recuperação é a do desastre.

---

## Parte 6 — O que PRECISA da chave de recuperação privada, e o que NÃO precisa

### NÃO precisa da privada de recuperação (o dia a dia inteiro)

- Fazer **deploy**.
- **Ler** um segredo para entregar ao container.
- **Criar, trocar, apagar** segredos.
- **Fazer backup** para o S3. Só precisa da **pública** de recuperação, para trancar.
- **Restaurar um backup no mesmo servidor** que ainda tem a chave dele. O servidor tenta a **própria chave
  primeiro** e consegue abrir. A de recuperação nem é consultada.

### PRECISA da privada de recuperação (só o desastre)

Quando o servidor **perdeu a chave dele** (a privada do servidor), e há dados trancados para ela:

- **Servidor novo e limpo**, recuperando a infra do git e os dados do S3: tudo foi trancado para o servidor
  **antigo** (que não existe mais) **e** para a recuperação. A única chave que sobrou é a de recuperação.
- **Disco morreu / VPS apagada.**
- **Alguém apagou `/etc/rustploy`** (e com isso o `age.key`).
- **A chave do servidor foi corrompida**, mesmo com o servidor ainda de pé.

Em todos esses, o assistente de recuperação do Rustploy **pede a privada de recuperação**, usa **só na
memória**, **recifra** os segredos para a chave **nova** do servidor novo, e **descarta** a privada da memória
(`begin_recovery` → `rekey` → `end_recovery`).

**Resumo da Parte 6:** a privada de recuperação **só** é usada **quando o servidor perdeu a própria chave**.
Se tudo correr bem, você **nunca** a usa. Mas **precisa** tê-la guardada **antes** do desastre.

---

## Parte 7 — "Precisa sair do servidor" e "quem recebe é o usuário" (a frase que confundiu)

Releia a analogia da chave reserva.

- A chave reserva só salva você **se estiver fora de casa**. Se ela ficar dentro, e a casa pegar fogo, as duas
  queimam.
- Com o Rustploy é igual: se a **privada de recuperação** ficar **gravada no servidor**, quando o servidor
  morrer ela morre junto, e a recuperação não serve para nada.

Então:

- **"A privada precisa sair do servidor"** = depois de criada, a privada de recuperação tem de ir **embora**
  do servidor e **não ficar gravada nele**.
- **"Quem a recebe é o usuário"** = para onde ela vai? Para **você**: um arquivo que você baixa, uma senha que
  você guarda no gerenciador de senhas, um papel impresso. **Você** é o "destino" da privada.

**É por isso que o servidor não cria a chave de recuperação sozinho e a esquece:** ele teria de **entregar** a
privada a alguém, e **só você** pode guardá-la fora. Se o daemon criasse sozinho e deixasse no servidor, seria
inútil.

**Resumo da Parte 7:** privada de recuperação = **sai do servidor** e **vai para o usuário guardar**. É por
isso que não dá para automatizar 100% sem alguém (você) segurando a privada no fim.

---

## Parte 8 — Um exemplo passo a passo, com um desastre

Vamos acompanhar a vida de um segredo, do início ao fim. Use "BD" para "senha do banco de dados".

**Dia 1. Instalação.**
- Você instala o Rustploy. O instalador cria a **chave do servidor** (Par A). A privada vai para
  `/etc/rustploy/age.key`. Você não fez nada.
- Você **ainda não** tem chave de recuperação. (`recovery_recipient` está comentado.)

**Dia 2. Você cria a chave de recuperação.**
- Você roda `rustploy-cli keygen`. Sai: `# chave pública: age1ABC…` e `AGE-SECRET-KEY-1XYZ…`.
- Você **guarda a privada** (`AGE-SECRET-KEY-1XYZ…`) no seu gerenciador de senhas. **Fora do servidor.**
- Você cola a **pública** (`age1ABC…`) no `config.toml`: `recovery_recipient = "age1ABC…"`.
- A partir daqui, **todo** segredo novo é trancado para as **duas** públicas.

**Dia 3. Você cadastra o segredo "BD".**
- O Rustploy tranca a senha para **servidor** + **recuperação**. No git/YAML aparece `ENC[age:…]`.
- No dia a dia, o servidor abre com a privada dele (`age.key`). Você nem lembra da recuperação.

**Dia 100. DESASTRE.** O disco da VPS morre. `/etc/rustploy/age.key` **sumiu**. O banco do Rustploy sumiu.
- O que sobrou: o **git** (com os `ENC[age:…]`) e o **S3** (com os backups). Estão **trancados**.
- A privada do **servidor antigo** foi embora com o disco.
- Mas os segredos também foram trancados para a **recuperação**. E você guardou essa privada fora.

**Dia 101. Recuperação.**
- Você sobe um servidor **novo e limpo**. Ele cria uma **chave do servidor nova** (Par A', diferente da antiga).
- No assistente de recuperação, você informa o git, o S3 e a **privada de recuperação** (`AGE-SECRET-KEY-1XYZ…`).
- O Rustploy abre os segredos com a privada de recuperação (na memória), **recifra** cada um para a chave
  **nova** do servidor e para a pública de recuperação, e depois **esquece** a privada.
- Pronto: o servidor novo funciona, e a chave de recuperação continua a mesma.

**E se você NÃO tivesse guardado a privada de recuperação?**
- Os segredos no git/S3 estão trancados **só** para as chaves antigas (servidor morto + recuperação perdida).
- **Ninguém** consegue abrir. Nem você, nem o desenvolvedor. Você teria que recadastrar todos os segredos à
  mão, e os dados cifrados do S3 ficariam **inacessíveis**.

**Resumo da Parte 8:** a chave de recuperação privada é o que separa "recuperei tudo" de "perdi tudo".

---

## Parte 9 — Onde cada coisa mora (mapa de arquivos)

| Coisa | Arquivo / lugar | Em que máquina | É segredo? |
|---|---|---|---|
| Privada do servidor (forma arquivo) | `/etc/rustploy/age.key` | servidor | **SIM, secretíssimo** |
| Privada do servidor (forma inline) | `config.toml` → `[secrets]` → `age_identity` | servidor | **SIM, secretíssimo** |
| Pública do servidor | não tem arquivo; aparece nos logs | servidor | não |
| **Pública** de recuperação | `config.toml` → `[secrets]` → `recovery_recipient` | servidor | **não** (é a pública) |
| **Privada** de recuperação | **gerenciador de senhas / pendrive / papel** | **FORA do servidor** | **SIM, secretíssimo** |
| Segredos cifrados `ENC[age:…]` | YAML de infra (vai para o git) | servidor + git | cifrados, ok |
| Backups `.rpdata` cifrados | o bucket S3 | S3 | cifrados, ok |

Observação conferida no código: os módulos de git (`state_git`) e de backup (`data_backup`) **não** copiam
o `config.toml` nem o `age.key`. Ou seja, a privada do servidor **não** vai para o git nem para o S3.
O que vai são os **segredos já cifrados**.

**Resumo da Parte 9:** só uma linha da chave de recuperação mora no servidor, e é a **pública**. A **privada**
de recuperação **nunca** deve estar no servidor.

---

## Parte 10 — Perguntas e respostas (as confusões que realmente apareceram)

**P: `age.key` é chave privada ou pública?**
R: **Privada.** É a privada **do servidor**.

**P: `age_identity` é a mesma coisa que `age.key`?**
R: **Sim.** Os dois são a **privada do servidor**. `age.key` é o arquivo; `age_identity` é a mesma privada
escrita dentro do `config.toml`. O servidor usa um ou outro.

**P: Então o servidor cria ou não cria chave sozinho?**
R: **Cria a chave DO SERVIDOR sozinho** (instalador ou daemon). **Não cria a chave DE RECUPERAÇÃO.** São duas
coisas diferentes. Quando eu disse "não cria", era sobre a **de recuperação**.

**P: `recovery_recipient` é pública ou privada?**
R: **Pública.** (A palavra "recipient" quer dizer "destinatário": para quem trancar.) A privada
correspondente fica com você, fora do servidor.

**P: Por que o servidor não pode simplesmente guardar a chave de recuperação privada também?**
R: Porque a recuperação existe para quando o servidor **some**. Se a privada estivesse nele, sumiria junto.

**P: Se eu nunca perder o servidor, preciso da chave de recuperação?**
R: No dia a dia, **não**. Mas o backup para o S3 **exige** a pública (`recovery_recipient`) para funcionar. E
ninguém sabe se vai perder o servidor. É um seguro.

**P: Perdi o `age.key` mas o servidor está de pé. E agora?**
R: Se o daemon reiniciar **sem** achar a privada e existirem segredos cifrados, ele **recusa subir** (para não
perder tudo). Você restaura a chave (de um backup do `age.key`) ou usa o assistente de recuperação com a
privada de recuperação.

**P: Posso usar a mesma chave para o servidor e para a recuperação?**
R: Tecnicamente daria, mas **não faz sentido**: o objetivo é a recuperação estar **fora** do servidor. Use
pares diferentes.

**P: Se alguém roubar o `config.toml`, perde algo?**
R: Depende. Se ele só tem `recovery_recipient` (pública) e `age_key_path`, não vaza chave. Se tiver
`age_identity` (a privada do servidor inline), **vaza**. Por isso o arquivo é `0640` e o dono é `rustploy`.

**P: A pública pode ser mostrada?**
R: **Sim.** É feita para isso. Só a **privada** é secreta.

**P: Como eu reconheço qual é qual?**
R: Pelo começo do texto. `age1…` = **pública** (pode mostrar). `AGE-SECRET-KEY-1…` = **privada** (esconda).

---

## Parte 11 — O que é a proposta do "assistente" (isto AINDA NÃO EXISTE)

> **Aviso:** esta parte descreve uma **ideia**, não algo implementado.

**O problema:** a chave de recuperação é o único par que **não nasce sozinho**. Quem não sabe dela só
descobre a falta **quando já perdeu o servidor**. Pior: o backup para o S3 **exige** a pública de recuperação,
então o usuário esbarra nisso na hora de ligar o backup.

**A ideia:** um assistente na primeira conexão (GUI e webui) que:

1. Detecta que **não há** chave de recuperação.
2. **Gera o par de recuperação** (Par B).
3. **Grava a pública** em `recovery_recipient` no `config.toml`. (A rotina `set_secrets_key_in_config` já
   existe e faz isso, hoje usada só na recuperação.)
4. **Entrega a privada ao usuário uma única vez** (baixar/copiar) e **não a guarda**.
5. Só deixa avançar depois que o usuário confirma que guardou.

**O ponto de atenção que ainda precisa ser **verificado** antes de implementar:** os segredos que já existiam
**antes** de a chave de recuperação ser criada foram trancados **só** para a chave do servidor. Para valerem
na recuperação, precisam ser **recifrados também** para a pública nova. Não conferi se o sistema já faz isso
automaticamente ao configurar a recuperação. **Isto é uma dúvida aberta**, não um fato.

**Decisões em aberto (do dono):** se o assistente é obrigatório ou dispensável; se aparece também nos
servidores já existentes (Chiquitos, Itemize); e que o comando fique só para admin e **fora** do alcance do
agente de IA (a resposta contém uma chave privada).

---

## Parte 12 — Erratas: o que foi dito errado antes (para você não se confundir de novo)

Na conversa que originou este documento, as seguintes afirmações estavam **erradas ou confusas**:

1. **"O daemon grava a identidade no `config.toml` e só cai para o `age.key` se não conseguir."**
   Isso vale **só** quando o daemon sobe **sem o instalador**. Num `.deb` normal, o **instalador** cria o
   `age.key` (arquivo) antes. Portanto, no `.deb`, o normal é o **arquivo**.
2. **"O `age.key` só aparece se não der para escrever o `config.toml`."** Idem: só vale para o daemon sozinho.
3. **"Ele não cria chave."** Era sobre a **chave de recuperação**, mas foi dito sem deixar claro, e parecia
   contradizer "ele cria a chave". **São duas chaves.** A do servidor é criada sozinha; a de recuperação não.
4. **"A pública da chave de recuperação entra em vigor na hora e isso já é feito hoje."** Hoje a pública de
   recuperação só é gravada pelo **fluxo de recuperação** (`begin_recovery`). **Ninguém** a cria sozinho no
   primeiro uso. A parte "entra em vigor na hora" referia-se à proposta do assistente, e foi dita de forma que
   parecia descrever o que já existe.
5. **"O `config.toml` carrega a privada do servidor."** Só carrega se for usado o `age_identity`. No `.deb`
   normal a privada fica no arquivo `age.key`, **fora** do `config.toml`.

Se algo neste documento contradisser algo dito antes na conversa, **vale este documento**.

---

## Parte 13 — Glossário rápido (uma linha cada, para consulta)

- **Segredo:** senha, token, chave de API que o Rustploy guarda.
- **Cifrar / trancar:** transformar o segredo em texto sem sentido.
- **Decifrar / abrir / destrancar:** voltar ao original.
- **`ENC[age:…]`:** um segredo trancado.
- **age:** o nome do sistema de cofre usado.
- **Par de chaves:** uma pública + uma privada, ligadas.
- **Chave pública (`age1…`):** só **tranca**. Pode mostrar.
- **Chave privada (`AGE-SECRET-KEY-1…`):** **abre**. Esconder sempre.
- **Identidade (`identity`):** o nome que o age dá à **chave privada**.
- **Recipient / destinatário:** o nome que o age dá à **chave pública** (para quem se tranca).
- **`age.key`:** arquivo com a **privada do servidor**.
- **`age_identity`:** a **privada do servidor** escrita dentro do `config.toml`.
- **`age_key_path`:** a linha do `config.toml` que **aponta** para o arquivo `age.key`.
- **`recovery_recipient`:** a **pública de recuperação**, no `config.toml`.
- **Chave do servidor:** o par A. Privada **dentro** do servidor. Automática.
- **Chave de recuperação:** o par B. Privada **fora** do servidor. Manual hoje.
- **`rustploy-cli keygen`:** comando que **gera um par** e imprime pública + privada.
- **Recifrar (`rekey`):** abrir um segredo e trancar de novo para chaves novas.

---

## Parte 14 — Mini-quiz (se acertar, entendeu)

1. Quantos **pares** de chaves o Rustploy tem? **Dois.** (Servidor e recuperação.)
2. Qual das duas é criada sozinha? **A do servidor.**
3. `age.key` e `age_identity` são a mesma coisa? **Sim**, a privada do servidor.
4. `recovery_recipient` é pública ou privada? **Pública.**
5. Onde deve ficar a **privada** de recuperação? **Fora do servidor, com você.**
6. Em que dia você usa a privada de recuperação? **No desastre**, quando o servidor perdeu a chave dele.
7. Um segredo trancado para servidor + recuperação abre com…? **Qualquer uma das duas privadas.**
8. Se perder o servidor e a privada de recuperação? **Os segredos não voltam.**
9. O que é `age1…`? **Pública.** E `AGE-SECRET-KEY-1…`? **Privada.**

Se errou alguma, releia a Parte correspondente: **3** (duas chaves), **4** (onde mora a privada do servidor),
**6** (quando usar a de recuperação) e **7** (por que sair do servidor).

## Parte 15 — O FLUXO COMPLETO: a vida das chaves, do instalador ao desastre

Esta parte conta a história inteira em ordem, dizendo **em cada passo** qual chave é usada. Vou usar estes
nomes curtos o tempo todo:

- **SERV-priv** = privada **do servidor** (`age.key` ou `age_identity`). Fica no servidor.
- **SERV-pub** = pública do servidor (calculada a partir da SERV-priv).
- **REC-pub** = pública **de recuperação** (`recovery_recipient`). Fica no servidor, só **tranca**.
- **REC-priv** = privada **de recuperação**. Fica **fora** do servidor, com você.

Lembrete que vale para tudo abaixo: **pública tranca, privada abre.**

### Fase 1. Instalação (uma vez)

1. O instalador (`postinst`) escreve `age_key_path = "/etc/rustploy/age.key"` no `config.toml`.
2. Ele roda `rustploy-cli keygen` e grava **a SERV-priv** em `/etc/rustploy/age.key` (permissão 0600).
   → **Chave criada: SERV (privada e pública).**
3. **REC não existe ainda.** `recovery_recipient` está comentado. Nenhuma REC-pub, nenhuma REC-priv.

*(Se o daemon sobe sem o instalador, ele mesmo cria a SERV-priv e a grava como `age_identity` no
`config.toml`. O resultado é o mesmo: existe uma SERV.)*

### Fase 2. Primeiro boot do daemon

1. O daemon procura a SERV-priv: primeiro `age_identity`, depois o arquivo `age_key_path`.
2. Achou → usa. Não achou e **já há segredos cifrados** → **recusa subir** (não cria chave nova, para não
   deixar os segredos ilegíveis). Não achou e **não há** segredos → cria uma.
3. Daqui para frente, **toda vez** que o daemon liga, ele carrega a SERV-priv na memória.

**Chave usada nesta fase: SERV-priv (carregada).**

### Fase 3. Você cria a chave de recuperação (HOJE: manual)

1. Você roda `rustploy-cli keygen` (no seu computador, de preferência, não no servidor).
2. Sai um par novo: **REC-pub** (`age1…`) e **REC-priv** (`AGE-SECRET-KEY-1…`).
3. Você guarda a **REC-priv fora do servidor** (gerenciador de senhas, pendrive, papel).
4. Você cola a **REC-pub** em `[secrets] recovery_recipient` no `config.toml` e reinicia o daemon.

→ **Chave criada: REC (privada com você, pública no servidor).**
A REC-priv **nunca** encosta no servidor. Só a REC-pub entra.

### Fase 4. Você cadastra um segredo (cifrar)

Exemplo: no projeto, você cria o segredo `DB_PASSWORD = "senha123"` na tela.

1. O daemon recebe `senha123` (texto puro, só em memória).
2. Ele **tranca** com **duas públicas ao mesmo tempo**: **SERV-pub** e **REC-pub** (se REC existir).
3. Grava `ENC[age:…]` no estado (o YAML de infra). `senha123` em texto puro **não** é gravado.

**Chaves usadas: SERV-pub + REC-pub (só as PÚBLICAS, para trancar).** Nenhuma privada é necessária para
cifrar. *(Se REC não existe ainda, tranca só para SERV-pub.)*

O mesmo vale para **outros valores sensíveis** que o Rustploy cifra: token de acesso do Git, token de
webhook, o segredo OAuth de um provedor Git e a **chave secreta do S3** (a do destino de backup). Todos são
trancados para SERV-pub + REC-pub ao serem gravados.

### Fase 5. Deploy: ler o segredo e injetar como variável de ambiente

Exemplo: você faz deploy de um serviço que usa `DB_PASSWORD`.

1. O daemon lê o `ENC[age:…]` do estado.
2. Ele **abre** com a **SERV-priv** (já carregada na memória). `resolve_env` faz isso.
3. Obtém `senha123` em texto puro (só na memória) e **injeta como variável de ambiente** no container.
4. A senha **não** é gravada em disco em texto puro.

**Chave usada: SERV-priv (para abrir).** A REC **não** participa. É o dia a dia.

O mesmo acontece quando o daemon precisa **usar** um token do Git (para clonar um repositório) ou a chave
secreta do S3 (para assinar um pedido ao bucket): ele abre com a **SERV-priv**, usa e descarta da memória.

### Fase 6. Sincronização com o Git (enviar o estado)

1. O daemon grava o estado da infra em YAML num repositório Git (`settings.yml` e os de projeto/serviço).
2. Os segredos vão **como `ENC[age:…]`** (já trancados). O texto puro **não** vai.
3. A **SERV-priv e a REC-priv NÃO vão para o git.** Só os segredos trancados vão.

**Chaves usadas: nenhuma nova.** O que vai ao git já está trancado para SERV-pub + REC-pub.

*(Quando o daemon **lê** de volta do git — por exemplo um `pull` — ele confere se **consegue abrir** cada
segredo com a chave que tem. Se não consegue, recusa, com a mensagem "não decifra com a identidade deste
servidor". É uma proteção.)*

### Fase 7. Backup de dados para o S3 (cifrar um arquivo)

Exemplo: o backup do volume do Postgres vai para o bucket S3.

1. O daemon empacota os dados num arquivo `.rpdata`.
2. Ele **tranca o arquivo inteiro** para **SERV-pub + REC-pub**.
3. Sem REC-pub configurada, o backup para o S3 **se recusa a rodar**, com a mensagem "configure a chave de
   recuperação…". (Backup para o S3 é sempre cifrado.)
4. Para falar com o S3, o daemon abre a **chave secreta do S3** com a **SERV-priv** e assina o pedido.
5. O arquivo trancado sobe para o bucket.

**Chaves usadas: SERV-pub + REC-pub para trancar o arquivo; SERV-priv para abrir a chave secreta do S3.**

### Fase 8. Restaurar um backup NO MESMO servidor (tudo normal)

1. O daemon baixa o `.rpdata` do S3.
2. Ele **abre** com a **SERV-priv**. (Foi trancado também para SERV-pub, então funciona.)
3. Restaura os dados.

**Chave usada: SERV-priv.** A REC **não** é necessária, porque o servidor ainda tem a chave dele. O daemon
tenta **sempre a SERV-priv primeiro**, e só recorre à REC-priv se ela falhar (Fase 10).

### Fase 9. O DESASTRE

A VPS foi perdida. O disco morreu. Junto sumiram:
- a **SERV-priv** (`age.key`) → **perdida para sempre**;
- o banco local do Rustploy;
- tudo o que estava naquele servidor.

O que **sobrou**:
- o **repositório Git** (com os segredos como `ENC[age:…]`, trancados para SERV-pub **antiga** + REC-pub);
- o **bucket S3** (com os `.rpdata`, trancados para SERV-pub **antiga** + REC-pub);
- a sua **REC-priv**, **guardada fora**.

Os segredos e backups estão **trancados**. A SERV-priv que os abria **não existe mais**. A única chave que
ainda abre é a **REC-priv**, porque eles foram trancados **também** para a REC-pub. **Este é o momento para
o qual a REC foi inventada.**

### Fase 10. A recuperação, passo a passo

1. Você sobe um **servidor novo e limpo** e instala o Rustploy. O instalador cria uma **SERV nova**
   (chamemos de **SERV′**), **diferente** da antiga. Esta SERV′-priv **não** consegue abrir o que foi trancado
   só para a SERV antiga.
2. Você abre o **assistente de recuperação**. Informa: o **Git** (e o token dele), a **REC-priv** e, se
   preciso, o **S3**.
3. O daemon guarda a **REC-priv só na memória** (nunca em disco) e grava a **REC-pub** no `config.toml`.
   *(Só a pública é gravada.)*
4. Ele busca o estado no Git. Para abrir cada segredo, tenta primeiro a **SERV′-priv** (falha) e depois a
   **REC-priv** (funciona).
5. **Recifra** (`rekey`) cada segredo: abre com a REC-priv e **tranca de novo** para **SERV′-pub + REC-pub**.
   Agora o servidor novo consegue abrir tudo com a **SERV′-priv** no dia a dia, e a REC continua valendo.
6. Para os dados, baixa os `.rpdata` do S3 e abre com a **REC-priv** (de novo: tenta a SERV′ primeiro, depois
   a REC). Restaura os volumes e os bancos.
7. No fim, a **REC-priv é descartada da memória**. O servidor volta a usar só a **SERV′-priv**.

**Chaves usadas nesta fase: REC-priv (para abrir o que é antigo, só na memória) e SERV′-pub + REC-pub (para
trancar de novo).** É o **único** momento em que a REC-priv entra em cena.

### Fase 11. Depois da recuperação (de volta ao normal)

O servidor novo funciona como o antigo: a **SERV′-priv** abre tudo no dia a dia, a **REC-pub** continua
trancando junto e a **REC-priv** volta a ser só a chave reserva. Se o servidor novo também morrer um dia, o
ciclo recomeça, com a **mesma REC-priv** guardada.

### A tabela-resumo do fluxo (qual chave em cada operação)

| # | O que acontece | Chave usada | Pública ou privada? | Quem participa |
|---|---|---|---|---|
| 1 | Instalação cria a chave do servidor | cria **SERV** | gera as duas | instalador |
| 2 | Boot do daemon carrega a chave | **SERV-priv** | privada | daemon |
| 3 | Você cria a chave de recuperação | cria **REC** | gera as duas | você (`keygen`) |
| 4 | **Cadastrar um segredo** (cifrar) | **SERV-pub + REC-pub** | **públicas** | daemon |
| 5 | **Deploy**: ler o segredo e injetar no container | **SERV-priv** | privada | daemon |
| 6 | Enviar o estado para o Git | nenhuma (já está cifrado) | — | daemon |
| 7 | **Backup para o S3** (cifrar o arquivo) | **SERV-pub + REC-pub** | **públicas** | daemon |
| 7b | Falar com o S3 (abrir a chave secreta do S3) | **SERV-priv** | privada | daemon |
| 8 | **Restaurar no mesmo servidor** | **SERV-priv** | privada | daemon |
| 9 | Desastre: SERV-priv some | — | — | — |
| 10 | **Recuperar em servidor novo**: abrir o antigo | **REC-priv** (memória) | privada | você + daemon |
| 10b | Recuperar: trancar de novo para o servidor novo | **SERV′-pub + REC-pub** | **públicas** | daemon |
| 11 | Dia a dia do servidor novo | **SERV′-priv** | privada | daemon |

### Duas regras que resumem o fluxo inteiro (decore)

1. **Para TRANCAR (cifrar), usam-se as chaves PÚBLICAS** (sempre as duas: SERV-pub e REC-pub). Isso acontece
   toda vez que se grava um segredo ou se faz um backup. Nenhuma privada é necessária.
2. **Para ABRIR (decifrar), usa-se uma chave PRIVADA.** No dia a dia é a **SERV-priv**. **Só no desastre** é a
   **REC-priv**.

### Onde a ideia do "assistente" entra nesse fluxo

A **Fase 3** é a única que depende de você e é manual hoje. O assistente proposto (Parte 11) automatizaria a
Fase 3: o daemon geraria o par REC, gravaria a REC-pub e **entregaria a REC-priv a você uma única vez** para
guardar. Todo o resto do fluxo (Fases 1, 2 e 4 a 11) já funciona sozinho.

**Atenção (dúvida aberta, ainda não verificada):** os segredos cadastrados **antes** da Fase 3 foram trancados
só para a SERV-pub. Não conferi se, ao configurar a REC-pub, eles são recifrados também para ela. Se não
forem, esses segredos antigos **não** entrariam na proteção da recuperação até serem salvos de novo. Convém
testar isso antes de implementar o assistente.
