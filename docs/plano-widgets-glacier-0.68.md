# Widgets novos do glacier-ui (0.63 → 0.68): o que dá pra aproveitar no rustploy

> **Status: IMPLEMENTADO em 2026-09-01** — as §1 a §4 e a §6 foram aplicadas,
> com os 58 testes verdes e as quatro telas alteradas conferidas na tela (ver
> §10, no fim). A §5 continua sendo só levantamento: nada dali foi feito.
>
> O texto abaixo foi escrito como estudo, antes da implementação, e está
> preservado no tempo verbal original — ele é o *porquê* de cada troca. A §10
> registra o que de fato mudou e o que a construção ensinou.

O `glacier-ui` saiu da **0.62.1** (a que o `rustploy-gui` usa hoje) para a
**0.68.0**, e nesse intervalo ganhou quinze widgets embutidos novos. Este
documento responde a uma pergunta só: **onde o rustploy escreve à mão hoje
alguma coisa que o motor passou a fazer sozinho?**

A resposta curta é que há três ganhos reais, um deles corrigindo um bug de
validação que existe hoje na tela de serviço, e uma porção de widgets novos que
não têm uso aqui. O resto do documento explica cada um com calma.

---

## 1. O bump em si é gratuito — e isso não era garantido

Antes de qualquer coisa: trocar `glacier-ui = "0.62.1"` por `"0.68.0"` no
`crates/rustploy-gui/Cargo.toml` **não quebra nada**. Foi medido:

```
cargo check -p rustploy-gui   →  ok, 36s
cargo test  -p rustploy-gui   →  58 passed (2 suites, 46s)
```

Isso merecia checagem porque o intervalo tem duas quebras declaradas no
CHANGELOG, e nenhuma das duas toca o rustploy:

| Quebra | Versão | Afeta o rustploy? |
|---|---|---|
| `app:` virou prefixo reservado de ação | 0.63.0 | **Não.** Nenhuma ação nossa se chama `app:algo`. |
| `<TimePicker>` perdeu `on_change`/`on_pick` | 0.68.0 | **Não.** O rustploy nunca usou `<TimePicker>`. |

Tem ainda uma mudança de aparência na 0.68 que o CHANGELOG descreve como
"afeta toda tela que não declarava padding": todo `<button>` sem `padding`
explícito nascia colado no texto, porque o motor sobrescrevia o default de 5px
do iced com zero. Varri as 19 classes de botão em uso no rustploy contra o
`views/styles/app.gss`: **18 delas declaram `padding`**, então não mudam. A
única exceção é `btn_secondary`, usada em **um** botão
(`service.gv:446`, "Salvar porta") — e essa classe nem sequer está definida no
GSS, ou seja, hoje o botão renderiza espremido e depois do bump passa a
respirar. É correção, não regressão.

Também confirmei que nenhum componente nosso colide de nome com os builtins
novos. Isso importa porque a 0.66 passou a aceitar tags minúsculas para os
builtins, e a 0.68 documenta um caso em que uma tela registrada com o nome de um
widget embutido causava um `SIGABRT` sem mensagem. Nossos nomes registrados são
`StatCard`, `ServiceCard`, `ProjectCard`, `rp-badge`, `TabButton`, `NavItem`,
`StateCell`, `PickerRow`, `TemplateRow`, `LoadingRow` — nenhum bate com
`card`, `badge`, `frame`, `groupbox`, `tabbar`, `avatar`, `toolbar`,
`statusbar`, `toolbutton`, `spinbox`, `slider`, `space`, `radio`, `radiogroup`,
`timepicker`. Passa longe.

**Conclusão da §1:** o bump é um one-liner seguro. Ele não é obrigatório para
nada — só é a porta de entrada para as §2, §3 e §4.

---

## 2. `<spinbox>`: onde hoje não existe validação nenhuma

Este é o achado mais concreto do estudo, e ele começa por um bug, não por um
widget.

### O bug

A aba **Advanced** e a aba **Health check** do detalhe de serviço
(`views/service.gv`) desenham mensagens de erro para os campos numéricos:

```xml
<text if="{erro_f_hc_interval}" not_equals="" class="err">
  {erro_f_hc_interval}
</text>
```

Existem seis dessas: `erro_f_hc_interval`, `erro_f_hc_timeout`,
`erro_f_hc_retries`, `erro_f_hc_start`, `erro_f_hc_status` e `erro_f_replicas`.

**Nenhuma delas é escrita em lugar nenhum.** Procurei em todo o
`crates/rustploy-gui` — Luau e Rust — e as seis chaves só aparecem sendo *lidas*
pelo próprio `service.gv`. São placeholders de uma validação que nunca foi
escrita.

O efeito prático: os campos são `<input>` de texto livre, e o handler faz

```lua
interval_secs = tonumber(Helpers.trim(ctx.f_hc_interval)) or cur.interval_secs,
retries       = tonumber(Helpers.trim(ctx.f_hc_retries))  or cur.retries,
local r       = tonumber(Helpers.trim(ctx.f_replicas))    or 1
```

Ou seja: digitar `abc` em RETRIES não dá erro — cai silenciosamente no valor
anterior. Digitar `-5` em REPLICAS manda `-5` para o daemon. O usuário não é
avisado de nada, porque a linha de erro está esperando uma chave que ninguém
preenche.

### O que o `<spinbox>` resolve

O `<spinbox>` (0.63, redesenhado na 0.64) é um campo numérico com as setas ▴▾
coladas nele, que **satura em `min`/`max`** e filtra a digitação para dígitos.
Ele é *builtin com comportamento*: a aritmética roda no `update` dele, em Rust.
Nenhuma linha de Lua do nosso lado.

```xml
<spinbox value="f_hc_retries"  min="1"  max="10"   width="90" />
<spinbox value="f_hc_interval" min="1"  max="3600" width="90" />
<spinbox value="f_replicas"    min="1"  max="20"   width="90" />
```

A chave continua sendo a mesma string no contexto — `f_hc_retries` etc. — então
**os handlers Luau não mudam uma linha**. O `tonumber(...) or cur.retries`
continua funcionando; ele só deixa de ser a rede de segurança de um campo que
aceita lixo, porque o lixo não entra mais.

### Onde aplicar

| Campo | Arquivo | min/max sugerido |
|---|---|---|
| `f_hc_interval` (INTERVAL s) | `service.gv` | 1 – 3600 |
| `f_hc_timeout` (TIMEOUT s) | `service.gv` | 1 – 3600 |
| `f_hc_retries` (RETRIES) | `service.gv` | 1 – 10 |
| `f_hc_start` (START PERIOD s) | `service.gv` | 0 – 3600 |
| `f_hc_status` (EXPECTED STATUS) | `service.gv` | 100 – 599 |
| `f_replicas` (REPLICAS) | `service.gv` | 1 – 20 |
| `dc_hours` (A CADA HORAS) | `home.gv` | 1 – 168 |
| `njob_hours` (A CADA HORAS) | `new_job_window.gv` | 1 – 168 |

Os três campos de porta (`f_port`, `f_host_port`, `f_gen_port`) também são
candidatos naturais (1 – 65535), mas ali vale pensar: porta é um número que o
usuário **digita inteiro**, não incrementa de 1 em 1. As setas seriam decoração.
Sugiro deixar de fora numa primeira leva.

Junto com a troca, as seis linhas `<text if="{erro_f_...}">` mortas podem sair —
ou, seguindo a regra de [comentar, não remover] do projeto, virar comentário com
um TODO explicando que a validação agora é do widget.

**Custo:** ~8 blocos de markup trocados, zero linhas de Luau, zero de Rust.

---

## 3. `<timeedit>`: os pares HORA/MINUTO viram um campo só

Hoje existem **três** lugares que pedem um horário e o fazem com dois campos de
texto livre lado a lado:

1. `home.gv` — Configurações → Manutenção → limpeza automática do Docker
2. `new_job_window.gv` — recorrência **Diária**
3. `new_job_window.gv` — recorrência **Semanal**

O markup é sempre este, repetido:

```xml
<row class="hc_grid">
  <column class="hc_field">
    <text class="label_cap">HORA (0-23)</text>
    <TextInput class="field_input" formControl="dc_hour" value="dc_hour"
      on_change="field:dc_hour" placeholder="3" />
  </column>
  <column class="hc_field">
    <text class="label_cap">MINUTO (0-59)</text>
    <TextInput class="field_input" formControl="dc_minute" value="dc_minute"
      on_change="field:dc_minute" placeholder="0" />
  </column>
</row>
```

Repare que o rótulo carrega a faixa válida entre parênteses — "(0-23)",
"(0-59)". É a confissão de que o campo aceita qualquer coisa e o usuário é quem
tem de saber o limite. E de novo o handler engole o erro em silêncio:
`hour = tonumber(ctx.dc_hour) or 0` — digitar `99` agenda a limpeza para
meia-noite sem avisar.

O `<timeedit>` da 0.68 é o `QTimeEdit`: um campo só, editado **por seções**.
Clicar na hora seleciona a hora (ela ganha realce), as setas ▴▾ mexem naquela
seção, e cada seção vira dentro de si — subir o minuto de 59 não empurra a hora.
Não dá para digitar, então não dá para digitar errado.

```xml
<text class="label_cap">HORÁRIO</text>
<timeedit value="dc_time" />
```

### A pegadinha: a chave é uma só, ISO

Aqui está a parte que exige trabalho de verdade. O `<timeedit>` grava **uma**
chave no formato `HH:MM`, enquanto o rustploy hoje tem **duas** chaves separadas
(`dc_hour` e `dc_minute`) que o Luau lê individualmente para montar o payload:

```lua
-- handlers/settings.luau
hour   = tonumber(ctx.dc_hour)   or 0,
minute = tonumber(ctx.dc_minute) or 0,
```

Então adotar o `<timeedit>` pede, em cada um dos três lugares:

- **na leitura** (`hour`/`minute` vindos da API → contexto): compor
  `ctx.dc_time = string.format("%02d:%02d", hour, minute)` em vez de gravar duas
  chaves. São as linhas `settings.luau:290-292` e `new_job_window.luau:88-90` /
  `124-127`;
- **na escrita** (contexto → payload da API): partir a string de volta em dois
  números. Um helper de três linhas em `scripts/fmt/time.luau`, que já existe,
  resolve para os três chamadores.

O contrato HTTP com o daemon **não muda** — ele continua recebendo
`{hour, minute}`. A conversão é toda do lado do cliente.

**Custo:** 3 blocos de markup (−6 campos, +3 widgets), ~1 helper novo e ~6
pontos de leitura/escrita no Luau. É a mudança mais trabalhosa do documento, e
também a que mais melhora a tela.

---

## 4. `<radiogroup>`: os seletores de dia da semana

Os dois seletores de dia da semana são hoje sete `<TabButton>` escritos à mão,
com um handler Luau só para gravar a escolha:

```xml
<row class="tabs">
  <TabButton label="Seg" current="{dc_weekday}" target="0" action="dc_weekday:0" />
  <TabButton label="Ter" current="{dc_weekday}" target="1" action="dc_weekday:1" />
  ... mais cinco ...
</row>
```

```lua
function dc_weekday(w: string): ()
    ctx.dc_weekday = w
end
```

O `<radiogroup>` (0.66) é exatamente isto: opções mutuamente exclusivas vindas
de uma coleção do contexto, e — o ponto — **ele grava a chave sozinho**, no
`update` dele em Rust. O handler Luau some.

```xml
<radiogroup value="dc_weekday" items="dias_da_semana" layout="row" />
```

com a coleção semeada uma vez, em Luau:

```lua
ctx.dias_da_semana = [[ [{"id":"0","label":"Seg"},{"id":"1","label":"Ter"}, ...] ]]
```

Isso troca 14 linhas de markup repetido por 2, e apaga dois handlers
(`dc_weekday` e `njob_weekday`).

**A ressalva honesta:** o `<radiogroup>` desenha *radio buttons* redondos, não a
fileira de pílulas que o `TabButton` desenha hoje. É uma **mudança de
aparência**, não uma troca invisível. Se a fileira de pílulas é o visual
desejado, a alternativa é o `<tabbar>` (0.65), que mantém a forma de abas e
também grava a chave sozinho — mas pede o par `value="dc_weekday"
active="{dc_weekday}"` e igualmente estiliza diferente do nosso `.tab_on`.

Minha recomendação: **fazer isto por último**, e só depois de olhar as duas
opções rodando lado a lado. Os outros dois itens são ganhos sem trade-off de
design; este não é.

---

## 5. O que também dá pra fazer, mas rende pouco

| Widget | Onde caberia | Por que não priorizar |
|---|---|---|
| `<space>` | As 5 classes `.np_spacer` / `.nav_spacer` / `.hspacer` (`width: fill`) usadas como `<column class="np_spacer" />` | Troca funciona e apaga 5 regras de GSS, mas o idioma atual já é claro. Ganho cosmético. |
| `<groupbox>` | O par `<text class="label_cap">TÍTULO</text>` + conteúdo, que aparece **110 vezes** | O `<groupbox>` desenha uma moldura com título; o `label_cap` é uma legenda em caixa alta sem moldura. Adotar mudaria a identidade visual de praticamente todas as telas. É um redesenho, não uma migração. |
| `<card>` / `<frame>` | `StatCard`, `ServiceCard`, `ProjectCard` | Os nossos já existem, já estão estilizados e o `ServiceCard` tem 11 props muito específicas. O builtin genérico não cobre. |
| `<toolbar>` / `<statusbar>` | A `.topbar` do `shell.gv` e o `topbar_status` | Faixas com aparência própria do Qt. Temos titlebar customizada (decorações desligadas) e um visual definido. |
| `<toolbutton>` | Os ~15 botões-ícone (`✕`, `▶`, `✎`, `≡`) | Já têm classes com hover (`copy_btn`, `env_del`). Movimento lateral. |
| **Slot nomeado** (0.67) | `ServiceCard`, que passa tudo por 11 props achatadas | Genuinamente interessante a médio prazo — deixaria o chamador escrever o corpo e as ações do card. Mas é refatoração de arquitetura de componente, não adoção de widget. Merece um plano próprio. |

E os que **não têm uso nenhum** aqui hoje: `<slider>` (não há faixa contínua na
UI), `<avatar>` (não há perfil de usuário), `<dateedit>` e `<datetimeedit>` (não
há campo de data — só horários recorrentes).

---

## 6. Paridade com a webui

A [regra dos dois clientes] vale aqui, e a boa notícia é que a webui tem
exatamente os mesmos buracos, com as mesmas soluções nativas do HTML:

| Item | GUI (glacier) | webui (`crates/daemon/webui/index.html`) |
|---|---|---|
| HORA/MINUTO livres | `<timeedit>` | `<input type="time">` — hoje é `type="text"` (linhas ~1017-1027, ~1322-1330) |
| Campos numéricos sem faixa | `<spinbox min max>` | `<input type="number" min max>` — só 2 dos campos usam `type="number"` hoje |
| Dia da semana | `<radiogroup>` | Já são 7 `<button>` com `:class` — equivalente ao nosso `TabButton` |

Ou seja: se a §2 e a §3 forem adiante, a contraparte na webui é de poucas linhas
e **melhora a validação lá também**. Vale fazer na mesma leva, não depois.

---

## 7. Ordem sugerida

1. **Bump para 0.68.0** — one-liner, já verificado (§1).
2. **`<spinbox>` nos 8 campos numéricos** + tirar as 6 mensagens de erro mortas
   (§2). Maior ganho por linha mexida, e corrige um bug real.
3. **`<input type="number" min max>` nos campos equivalentes da webui** (§6).
4. **`<timeedit>` nos 3 pares HORA/MINUTO** + o helper de conversão em
   `fmt/time.luau` (§3), e `<input type="time">` na webui.
5. **`<radiogroup>` nos 2 seletores de dia da semana** (§4) — só depois de
   decidir a aparência.
6. Deixar `<space>`, slot nomeado e o resto para um segundo momento (§5).

## 8. Portão de verificação

`cargo test` verde **não basta** para nada disto: são todas mudanças de UI, e o
que muda é o pixel. Antes de dar qualquer etapa por pronta, rodar a GUI de fato
— seja `cargo run -p rustploy-gui`, seja o smoke headless com Xvfb — e olhar as
telas alteradas. Vale lembrar que o app aberto pelo `.deb` lê
`/usr/share/rustploy`, não o workspace: uma edição de `.gv` só aparece via
`cargo run` ou `make deb-gui`.

## 9. Estado do working tree neste momento

O bump da §1 está **aplicado e não commitado**, porque foi preciso aplicá-lo
para medir:

```
 M Cargo.lock
 M crates/rustploy-gui/Cargo.toml
```

É `git checkout` de dois arquivos se a decisão for não seguir agora.

---

# 10. O que foi implementado (2026-09-01)

## O que mudou, por arquivo

| Arquivo | O quê |
|---|---|
| `crates/rustploy-gui/Cargo.toml` | `glacier-ui` 0.62.1 → 0.68.0 |
| `views/service.gv` | 6 `<input>` → `<spinbox>`; as 6 linhas `erro_f_*` mortas saíram, com comentário explicando o porquê |
| `views/home.gv` | `dc_hours` → `<spinbox>`; par HORA/MINUTO → `<timeedit>`; 7 `<TabButton>` → `<radiogroup>` |
| `views/new_job_window.gv` | `njob_hours` → `<spinbox>`; **dois** pares HORA/MINUTO → `<timeedit>`; 7 `<TabButton>` → `<radiogroup>` |
| `views/scripts/fmt/time.luau` | `hm_join`/`hm_split` (ponte "HH:MM" ↔ `{hour, minute}`) e `WEEKDAYS_JSON` |
| `views/scripts/fmt.luau` | reexporta os três |
| `views/scripts/handlers/settings.luau` | usa `dc_time`; `dc_weekday()` comentado (órfão) |
| `views/scripts/new_job_window.luau` | usa `njob_time`; `njob_weekday()` comentado (órfão) |
| `views/scripts/handlers/connection.luau` | semeia `ctx.weekdays` no `init()` |
| `tests/templates_render.rs` | semeia `dc_time`/`njob_time`/`weekdays` no lugar das chaves antigas |
| `webui/index.html` | 7 campos ganharam `type="number" min max`; 2 pares HORA/MINUTO → `type="time"`; dia da semana livre → `<select>` |
| `webui/fmt.js` | `hmJoin`/`hmSplit`, porta das funções Luau |
| `webui/app.js` | estado `njobTime`/`dcTime` no lugar de hora+minuto, nos 6 pontos de leitura/escrita |

Resumo: **14 arquivos, ~250 linhas adicionadas e ~190 removidas.**

## Decisões que a construção forçou, e que o estudo não previa

**1. O `<spinbox>` não repassava `class` nem `form_control`** — e isso virou
uma mudança no glacier-ui, não uma limitação a conviver. Ver a §11.

Na primeira leva a limitação foi aceita, com esta análise: o `<TextInput>` que
o widget monta por dentro aceitava só `value`, `onChange`, `placeholder` e
`width`. O custo visual era nulo (`.field_input` aqui é só
`width: fill; padding: 13 14`; o fundo e a borda vêm do estilo padrão do
motor), e o custo de comportamento parecia pequeno.

**Correção sobre o que escrevi então:** eu disse que o campo perdia "a ordem de
tabulação". Está errado, e a diferença importa. A travessia por **Tab** é um
listener global do motor (`focus_next`, `lib.rs`) que percorre todo widget
focável sem olhar para `formControl` — o campo do spinbox sempre foi alcançado
por ela. O que `form_control` liga é o **Enter**: submeter o formulário e
avançar para o campo seguinte. Sem ele, o spinbox engolia o Enter.

Segue valendo o registro de que `Form::sync_to_context` só é chamado por um
`impl Component` em Rust, e o rustploy não tem nenhum — os handlers leem
`ctx.f_hc_*` direto, então nunca houve risco de o `<Form>` sobrescrever a chave.

**2. As portas ficaram de fora, como o estudo sugeriu.** `f_port`,
`f_host_port` e `f_gen_port` continuam `<input>` de texto. Porta é um número
que se digita inteiro, não que se incrementa de 1 em 1 — as setas seriam
decoração, e a faixa 1–65535 num spinbox não ajuda ninguém.

**3. `WEEKDAYS_JSON` mora em `fmt/time.luau`, não em cada tela.** O
`<radiogroup>` lê as opções de uma *chave* do contexto (o `for-each` do motor
lê chave, não texto), então a lista precisava existir em dois contextos
diferentes — a janela principal e a janela "Novo job", que são motores
separados. Uma constante só, semeada nos dois `init()`.

**4. A semeadura de `weekdays` ficou no `init()`, não no `dc_apply_config`.** É
uma lista fixa, não depende da config carregada do daemon; se o load falhasse,
semeá-la lá dentro deixaria o grupo vazio.

**5. O `id` do dia da semana é o índice que o daemon usa** (`0` = segunda), e
não um nome. Assim ele viaja do `<radiogroup>` para o payload de
`Recurrence::Weekly` sem conversão nenhuma.

## O portão visual: como foi feito, e por que não dá pra pular

`cargo test` diz que a árvore de elementos monta; não diz como ela fica. As
quatro telas alteradas foram abertas de verdade, headless, e conferidas por
captura:

| Tela | O que se confirmou |
|---|---|
| Novo job → Semanal | `<radiogroup>` com "Qua" marcado; `<timeedit>` em `03:00` com a seção da hora em destaque e as setas ▴▾ |
| Novo job → Intervalo | `<spinbox>` "A CADA QUANTAS HORAS" = 6, com a coluna ▴▾ colada ao campo |
| Serviço → Healthcheck | os 5 spinboxes (status 200, interval 30, timeout 5, retries 3, start 10) alinhados no `hc_grid` |
| Serviço → Advanced | `<spinbox>` REPLICAS = 1 |
| Settings → Manutenção | `<radiogroup>` com "Sex" marcado + `<timeedit>` em `03:00` |

**A receita**, porque ela não é óbvia e vale repetir: as telas alteradas ficam
atrás do login, e o login lembrado tem credenciais de **produção** — conectar
para testar está fora de questão. A saída é um `examples/` temporário no
`rustploy-gui` que usa `GlacierDaemon::new()`, registra o `.gv` direto e semeia
o contexto à mão (`define_data`), exatamente como o `templates_render.rs` faz,
mas abrindo a janela em vez de só renderizar. Daí `Xvfb :77` (com
`WAYLAND_DISPLAY` **removido** do ambiente, senão o iced ignora o `DISPLAY`),
`import -window root` para a captura, e `python-xlib` (`xtest.fake_input` com
botão 5) para rolar até a seção de interesse — `xdotool` não está instalado
nesta máquina. O harness foi **apagado depois**: ele não é parte da entrega,
só do portão.

## O que continua aberto

Tudo da §5 — `<space>`, `<groupbox>`, `<card>`/`<frame>`,
`<toolbar>`/`<statusbar>`, `<toolbutton>` e o **slot nomeado** para o
`ServiceCard`. O slot nomeado é o único com valor de arquitetura de verdade
(hoje o `ServiceCard` recebe 11 props achatadas) e merece um plano próprio.

Uma dívida que este trabalho **não** pagou: a aparência do `<radiogroup>`. Ele
desenha radio buttons redondos, e o que estava ali antes era uma fileira de
pílulas (`TabButton`). Nas capturas ficou coerente com o tema e legível, mas é
uma escolha estética que continua reversível — os handlers `dc_weekday` e
`njob_weekday` foram **comentados, não apagados**, justamente para que voltar
seja uma edição de markup e não uma arqueologia.


---

# 11. Segunda leva: o que virou mudança no glacier-ui (0.69.0)

A limitação do item 1 da §10 não sobreviveu à pergunta seguinte — "e se o
spinbox simplesmente repassasse?". A resposta foi que ele podia, e que por trás
dela havia um bug maior.

## O bug que apareceu no caminho

`class` escrita na tag de **qualquer** componente — builtin da lib ou do app —
era um **no-op silencioso**. A classe era lida pelo parser (é atributo genérico
de nó), viajava no mapa de props e depois ninguém a usava: sem erro, sem aviso,
sem log. `<spinbox class="campo_num"/>` simplesmente não pintava nada.

A 0.69.0 corrige isso: a classe do **uso** aplica na **raiz expandida** do
template, numa escada de especificidade explícita —

```
tag de componente < tag builtin < classe do template <
classe do USO < id do template < inline do template
```

— ou seja, **a classe do uso vence as classes do template e perde para os
atributos inline dele**. Detalhe que quase passou: a classe do uso precisou
entrar na *chave do cache* de componente, senão um `class="{estado}"` dinâmico
serviria a árvore velha para sempre (a interpolação acontece no quadro de fora,
então a dependência não estaria entre as que o cache guarda).

## O que o rustploy ganhou

O `<SpinBox>` ganhou `field_class` e `form_control`, e os 8 spinboxes daqui
passaram a usar os dois:

```gv
<spinbox field_class="field_input" form_control="f_hc_interval"
         value="f_hc_interval" min="1" max="3600" width="90" />
```

- **`field_class="field_input"`** alinha a altura do campo com os `<input>`
  vizinhos da mesma tela — no grid de health check, o HTTP PATH fica logo acima
  dos numéricos, e antes havia um degrau visível entre eles.
- **`form_control`** devolve o Enter: submeter e avançar. Eram 6 buracos no
  fluxo de teclado da aba Health check.

`field_class` não se chama `class` porque, com a correção acima, `class` num
spinbox passou a significar "o widget inteiro" — a `Row`, campo mais degraus,
que é o que estilizar um `QSpinBox` significa no Qt. As duas coisas são
legítimas e diferentes.

O plano completo, com as decisões de projeto e o que ficou de fora, está em
`glacier-ui/PLANO_CLASS_EM_COMPONENTE.md`.


---

# 12. Terceira leva: o `<timeedit>` ganhou teclado (glacier-ui 0.70.0)

O widget da 0.68 editava por seções **só no mouse** — "não dá para digitar no
campo" era uma limitação declarada no CHANGELOG dela. Na prática, isso deixava
os três horários do rustploy (limpeza automática, job diário, job semanal) numa
posição pior que a dos campos de texto que substituíram: dava para ler, mas
ajustar exigia mirar numa seta de 9px.

A 0.70.0 fecha isso. Com uma seção selecionada:

| tecla | o que faz |
|---|---|
| ▲ / ▼ | passo na seção selecionada |
| ← / → | troca de seção, sem alterar valor |
| `0`–`9` | digita na seção, avançando sozinha quando enche |

Digitar `0930` numa hora atravessa hora e minuto sem tocar no mouse. **O
rustploy não precisou de mudança nenhuma** — os três `<timeedit>` daqui ganham
o comportamento só pelo bump.

## Um limite que vale conhecer

Com um campo de texto de linha única focado, **▲▼ ainda alcançam** uma seção de
`<timeedit>` que tenha sido selecionada antes e não largada por um clique. O
`text_input` do iced não usa ▲▼, então o evento chega ao motor como "ninguém
quis" e é indistinguível de uma tecla livre. Algarismos e ← → **não** vazam (o
campo focado os captura), e clicar em qualquer outro widget larga a seção.

Isso importa para a aba **Manutenção** e a janela **Novo job**, que têm campo de
texto e `<timeedit>` na mesma tela: se o usuário clicar numa seção do horário e
depois for digitar noutro campo, o ▲▼ ainda mexe no horário até ele clicar em
algo. Fechar de vez exige o widget virar um nó focável de verdade no iced, o que
é outra obra e não foi feita.

## Como isso foi verificado

Não dava para verificar teclado antes: sob `Xvfb` **nenhuma tecla chega ao
app** — sem window manager, o winit nunca se considera focado e descarta o
teclado (nem `setxkbmap` nem `set_input_focus` resolvem). A receita que
funciona é um X **aninhado no display real** com um WM de verdade:

```bash
DISPLAY=:1 Xephyr :83 -screen 900x760 -ac &
env -u WAYLAND_DISPLAY DISPLAY=:83 kwin_x11 &
env -u WAYLAND_DISPLAY DISPLAY=:83 ./target/debug/examples/timepicker &
```

Com ela: clicar na hora e teclar ▲▲ levou `13:45:02` a `15:45:02`; digitar
`0730` levou a `07:30:02`, com a seleção avançando hora → minuto → segundos
sozinha. `Xvfb` continua servindo para conferência **só visual**.
