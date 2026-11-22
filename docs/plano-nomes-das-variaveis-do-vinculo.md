# Plano: escolher o nome de cada variável que o vínculo entrega

**Status:** implementado (2026-11-22) na v0.1.89 (daemon 0.1.74, app 0.1.82), entrega autônoma.
Continuação de [`plano-recursos-e-vinculos.md`](plano-recursos-e-vinculos.md).

## O problema

Um vínculo (`api` usa `s3`) entrega ao serviço variáveis com nomes fixos: `<PREFIXO>_HOST`,
`<PREFIXO>_PORT`, `<PREFIXO>_URL` e, com recurso, `<PREFIXO>_BUCKET` ou `<PREFIXO>_DATABASE`. Só o
prefixo podia ser trocado.

Isso basta quando o código lê nomes que seguem esse formato (`DATABASE_URL`, `REDIS_URL`), mas não
quando ele lê outros. O caso real é o Chiquitos: o `api` lê `S3_ENDPOINT` e `S3_BUCKET_NAME_1`, e
nenhum prefixo gera esses nomes. A saída era mudar o código do app para seguir o Rustploy, ou continuar
digitando as variáveis à mão — e aí o vínculo não ajuda em nada.

## O modelo

Cada vínculo ganha um mapa opcional **`env_names`**: para cada variável que ele entrega (a chave é o
sufixo: `HOST`, `PORT`, `URL`, `BUCKET`, `DATABASE`), o nome com que ela entra no container.

```yaml
links:
  - service_id: svc_s3
    resource: { name: chickie-uploads, public_read: true }
    env_names:
      URL: S3_ENDPOINT        # renomeada
      BUCKET: S3_BUCKET_NAME_1
      PORT: ""                # string vazia = não entregar
```

- Sufixo **fora** do mapa: vale o nome padrão (prefixo + `_` + sufixo), como hoje. Por isso o prefixo
  continua existindo: ele é o atalho que gera os nomes de tudo o que não foi renomeado.
- Valor **vazio**: a variável não é entregue.
- Um vínculo sem `env_names` funciona exatamente como antes. Nada muda para quem não mexer.

**Regras que o daemon confere ao gravar:** sufixo conhecido; nome válido de variável (`A-Z`, `0-9` e
`_`, sem começar por dígito, o mesmo teste do prefixo); e um nome **renomeado** não pode repetir outro
nome entregue pelos vínculos do mesmo serviço (duas variáveis com o mesmo nome, uma apagaria a outra).
A checagem de repetição só olha os nomes renomeados: dois vínculos antigos com o mesmo prefixo, que já
existem gravados, continuam gravando (decisão conservadora: não travar o que hoje funciona).

**Precedência:** não muda. As variáveis digitadas no projeto ou no serviço **ganham** das do vínculo.
Isso vale também para um nome renomeado: renomear para `S3_ENDPOINT` enquanto o projeto tem
`S3_ENDPOINT` digitada não muda nada no container. Por isso a tela avisa (abaixo).

**Versão anterior lendo o novo:** o campo novo é `#[serde(default)]` e o anterior não usa
`deny_unknown_fields`: uma v0.1.88 que leia um spec com `env_names` ignora o campo e entrega os nomes
padrão. Nenhuma migração de dados: o YAML de infra e o SQLite guardam o spec como estava, com um campo
a mais quando alguém renomeia.

## A tela (GUI e webui, as duas)

Na aba Avançado, seção **USA (N)**:

1. **Cada vínculo** mostra as variáveis com os nomes efetivos (`S3_ENDPOINT · S3_HOST · S3_BUCKET_NAME_1`),
   e ganha um botão **Editar**. Antes, para mudar um vínculo era preciso desfazer e refazer.
2. **Abaixo dele**, quando algum nome entregue também está digitado no projeto ou no serviço, um aviso:
   *"S3_ENDPOINT já está digitada (projeto): a digitada ganha."*
3. **O formulário** (o mesmo de adicionar) ganha o bloco **Nomes das variáveis**, que aparece quando um
   serviço é escolhido: uma linha por variável, com o campo do nome (em branco = o padrão, mostrado como
   dica) e uma chave **Entregar**. As linhas de `BUCKET` e `DATABASE` só aparecem para o tipo de
   serviço que as entrega, como os campos de recurso.
4. **Editar** carrega o vínculo no formulário (no lugar do seletor aparece "Editando o vínculo com s3"
   e um **Cancelar**), e o botão vira **Salvar vínculo**. Salvar substitui o vínculo inteiro.

Editar não é destrutivo (nada se perde que não esteja na tela), então não pede confirmação. Desfazer
continua pedindo.

## O que entra nesta entrega

Tudo acima: modelo, validação, entrega no deploy, as duas telas, docs pt/en, manual do agente.

## O que fica para depois

- **Referências na tela de variáveis** (`S3_ENDPOINT = ${{s3.URL}}`, estilo Railway): mais poderoso,
  mas mexe em todo lugar que resolve variáveis. Só se a renomeação não bastar.
- O lado **Quem usa este serviço** continua só adicionando vínculos com os nomes padrão; renomear é
  feito na tela de quem usa.
- Trocar as variáveis do Chiquitos por vínculos renomeados: é uma mudança em produção, fica para o
  dono pedir.
