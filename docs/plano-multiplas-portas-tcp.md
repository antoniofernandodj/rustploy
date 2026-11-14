# Plano: mais de uma porta TCP por serviço

> **Estado (2026-11-13): implementado (Fases 0 a 4) e publicado.** Decisões tomadas com os padrões sugeridos no
> fim deste documento: a porta que leva à `port` é espelhada no campo antigo (sem prazo para tirar: custa quase
> nada e protege clientes e daemons antigos); limite de 8 portas por serviço; rótulo opcional por porta
> (preenchido pelas sugestões da Fase 3). Testes: modelo (shared), alocação e sugestões (daemon), formatação
> (Luau) e o editor da webui num Chrome real. **Não visto ainda:** as telas num daemon instalado, e as capturas
> da aba Domains nas docs (`interface/servico-configuracao`) continuam as antigas.

## O problema, em linguagem simples

Hoje cada serviço pode ter **uma** "Porta TCP do host" (na aba Domínios). Ela liga uma porta da máquina
(ex.: `20002`) à **porta principal** do serviço (o campo `port` do spec), e não dá para escolher outra.

Isso funciona para quem tem uma porta só (um Postgres na 5432, um Redis na 6379). Não funciona para quem
tem duas, e vários serviços comuns têm:

| Serviço | Porta principal | A outra porta | O que fica de fora hoje |
|---|---|---|---|
| RustFS / MinIO | 9000 (API S3) | 9001 (console web) | o console |
| RabbitMQ | 5672 (AMQP) | 15672 (painel de gestão) | o painel |
| Kafka + UI, Mongo + Express… | a do protocolo | a da interface | a interface |

Foi o que aconteceu no teste do `s3`: a `20002` abriu a API S3 (o "Access Denied" em XML é ela recusando
um acesso sem credencial), e o console na 9001 não tinha como ser alcançado por porta.

## O que já dá para fazer hoje (sem este plano)

Se a segunda porta é uma **interface web**, ela já tem caminho: na mesma aba Domínios, "Adicionar domínio"
aceita uma **porta do container** por domínio. Um domínio apontando para a 9001 resolve o console do
RustFS. Este plano é para quando a segunda porta **não** é HTTP (ou não se quer domínio, por exemplo numa
máquina local ou atrás de VPN): um segundo banco de protocolo, uma porta de métricas, uma porta de
replicação.

## A ideia

Fazer com a porta TCP o mesmo que já foi feito com os domínios: em vez de **um** valor, uma **lista**, e
cada item diz **para qual porta do container** ele vai.

```
Porta do host   →  Porta do container
20002           →  (padrão do serviço: 9000)
20003           →  9001
```

Na tela, a caixa "Porta TCP do host" vira uma lista com "Adicionar porta", igual à lista de domínios.

### Por que copiar o desenho dos domínios

Porque ele já foi testado neste projeto. Os domínios eram um campo só (`domain`) e viraram uma lista
(`domains`), cada um com sua porta do container, **sem quebrar** servidores, repositórios git e clientes
antigos: o campo antigo continuou existindo, só como leitura, e uma função (`domain_routes()`) junta os
dois. O plano é repetir exatamente isso: o mesmo jeito de pensar, já conhecido de quem mexe no código.

### Por que a porta do container é opcional

Pelo mesmo motivo do domínio: vazio quer dizer "a porta principal do serviço". Assim, a porta que
existe hoje em cada serviço continua significando exatamente o que significa, sem nenhuma conversão.

## O que muda para quem usa

- A aba Domínios mostra uma lista de portas TCP, cada uma com "porta do host" (ou **Automática**, como
  hoje) e "porta do container" (opcional).
- A tela de Ingress mostra uma linha por porta, não por serviço.
- A aba Conexão continua mostrando **uma** URL externa: a da porta que leva à porta principal do serviço
  (se não houver, a primeira da lista).
- Serviços que já existem **não mudam**: a porta deles vira o primeiro item da lista, apontando para a
  porta principal, como antes.

## Cuidados (o que pode dar errado e como o plano evita)

1. **Firewall.** Cada porta da lista é liberada no firewall ao subir e fechada ao sair da lista ou ao
   apagar o serviço. Hoje isso é feito para uma porta; passa a ser feito comparando a lista antiga com a
   nova (o que saiu fecha, o que entrou abre), mantendo a regra de nunca fechar uma porta que outro
   serviço ainda usa.
2. **Alocação automática.** Duas portas "Automática" no mesmo serviço precisam receber números
   diferentes. A alocação passa a considerar também as portas já reservadas na própria lista.
3. **Mesma porta repetida.** Uma porta do host repetida (na lista ou entre serviços) é recusada ao
   salvar, com mensagem clara, como hoje.
4. **Versões misturadas.** Um cliente antigo (GUI/webui de antes) conectado a um daemon novo só enxerga o
   campo antigo. Para não sumir nada da tela dele, o daemon continua **espelhando a primeira porta** no
   campo antigo por algumas versões. Um daemon antigo lendo um repositório git escrito por um novo
   ignora a lista (o formato aceita campos desconhecidos) e fica só com a primeira porta: perde as
   extras, mas não quebra. Como cada repositório é de um servidor só, isso é raro.
5. **Stack Compose com vários containers.** Hoje, numa stack, **um** container recebe o tráfego externo
   (escolhido pelo `ingress_service` ou por quem expõe a porta pedida). Este plano mantém isso: todas as
   portas da lista vão para esse mesmo container. Portas que estão em containers **diferentes** da mesma
   stack ficam fora deste plano (seria outro: "um alvo por porta").
6. **"Exportar serviço" e cópia entre servidores.** Hoje a porta exportada volta como "Automática" no
   destino (para não colidir com outra porta de lá). Cada item da lista passa a fazer o mesmo, mantendo
   a porta do container.

## Fases

Cada fase termina com testes passando e pode ser revisada sozinha.

### Fase 0: o modelo (`rustploy-shared`)

- `ServiceSpec.tcp_ports: Vec<TcpPortRoute>` com `#[serde(default)]`, onde
  `TcpPortRoute { host_port: u16, container_port: Option<u16>, label: Option<String> }` (`host_port = 0` =
  "Automática"; `label` é só para a tela: "console", "painel").
- `ServiceSpec::tcp_routes()`: a lista, ou, se vazia, o `host_port` antigo virando um item com
  `container_port: None`. Todo código novo lê por aqui, nunca o campo antigo.
- `ServiceSpec::materialize_tcp_ports()`: move o campo antigo para a lista ao salvar, e espelha a
  primeira porta de volta no `host_port` (item 4 dos cuidados).
- `ingress_container_ports()` passa a incluir a porta do container de cada item.
- Testes: legado sem lista, lista nova, espelhamento, `container_port` vazio = porta principal.
- Sobe a versão menor do `rustploy-shared` (é publicado no crates.io).

### Fase 1: o daemon (`rustploy-daemon`)

- **Validação e alocação** (`ingress/external_ports.rs`): resolve cada item, aloca as "Automática" sem
  repetir, recusa repetidas; `used_ports` passa a contar todos os itens de todos os serviços.
- **Rotas no deploy**: os 5 lugares que hoje registram a porta única (`compose_flow.rs`, `promote.rs`,
  `staging.rs`, `rollback.rs`, `recovery.rs`) passam a percorrer `tcp_routes()`, cada um com
  `ip:porta_do_container`.
- **Firewall** (`services/create.rs`, `update.rs` `sync_firewall`, `delete.rs`): diferença entre a
  lista antiga e a nova.
- **Healthcheck** (`executor/healthcheck.rs`): a condição "tem domínio ou porta" passa a olhar a lista.
- **Exportar/importar e manifesto** (`iac/service_bundle.rs`, `iac/manifest.rs`, `service_import.rs`):
  a lista, com cada porta do host virando "Automática" no destino.
- **Conexão** (`ServiceConnectionInfo`): escolhe a porta que leva à porta principal.
- **Manual do agente MCP** (`api/agent/manual.md`): documenta a lista.
- Testes de alocação (duas automáticas, repetida, colisão com outro serviço) e de diferença do firewall.

### Fase 2: as duas interfaces (GUI e webui juntas)

Pela regra do projeto, entra nas duas ao mesmo tempo.

- **GUI**: `service/domains.component.gv` (a lista no lugar da caixa única), `handlers/services.luau`,
  `format/service_detail.luau`, `format/dashboard.luau`, `home/ingress.component.gv`,
  `handlers/connection.luau`, traduções.
- **webui**: `index.html` (aba Domínios), `screens/service_detail.js`, `screens/ingress.js`,
  `format.js` (`hostPortRows`, `externalUrl`), traduções.
- O wizard continua criando **uma** porta "Automática" (a de hoje).

### Fase 3: sugestões de porta do container

**A ideia (do dono, 2026-11-13).** Ao adicionar uma porta, o campo "porta do container" sugere as portas que o
serviço tem, para não ser preciso saber de cabeça que o console do RustFS é a 9001. Em vez de um botão que
alterna a cada clique (que esconde as opções), uma **lista de sugestões** no próprio campo, cada uma com nome e
origem:

```
Porta do host        Porta do container
[ Automática ]  →   [ 9001          ▾ ]
                      9000  API S3      (escutando agora)
                      9001  console     (escutando agora)
```

**De onde vêm as sugestões.** Ler só o compose não basta: o do RustFS e o do RabbitMQ criado pelo assistente
não declaram `ports:` (o RustFS só tem `RUSTFS_ADDRESS`/`RUSTFS_CONSOLE_ADDRESS`). O daemon junta quatro fontes:

1. **O que o container escuta agora** (lido de dentro dele, `/proc/net/tcp` e `tcp6`). A mais fiel. Imagens
   mínimas sem `cat` não deixam ler: aí esta fonte some, sem erro.
2. **As portas que a imagem declara** (`EXPOSE`, no `Config.ExposedPorts` da imagem). Vale com o serviço parado.
3. **As portas do compose** (`ports:` e `expose:`), quando existem.
4. **Uma tabela por produto** (o spec já tem `product`): dá **nome** às portas conhecidas (RustFS: 9000 "API
   S3", 9001 "console"; RabbitMQ: 5672 "AMQP", 15672 "painel", 15692 "métricas"; Postgres 5432; Redis 6379…) e
   marca as **internas** (RabbitMQ 25672 e 4369, de cluster), que vão para o fim da lista com o aviso "interna".

Cada sugestão traz porta, nome (se a tabela souber) e origem. Portas que já estão na lista do serviço ficam de
fora. A porta do host continua "Automática" por padrão (uma porta livre da faixa).

**Peças.** Um comando novo, `ServicePortSuggestions { service_id }` → lista de `{ port, label, source,
internal }`; os dois clientes o chamam ao abrir o editor de portas.

### Fase 4: documentação e índice

- `rustploy-docs`: `guias/dominios-e-tls.mdx`, `interface/servico-configuracao.mdx`,
  `referencia/manifesto.mdx`, FAQ (pt e en). Um exemplo com o RustFS (API + console) e o RabbitMQ
  (AMQP + painel).
- `make index`.

## Fora do plano

- Portas UDP.
- Uma porta indo para um container diferente da stack (item 5 dos cuidados).
- Balancear uma porta entre portas diferentes do container.

## Decisões que são do dono

1. **Espelhar a primeira porta no campo antigo** (item 4): por quantas versões? Sugestão: até a próxima
   versão em que GUI e webui antigas já não forem usadas, depois remover o espelho.
2. **Limite de portas por serviço**: sugestão de 8, para uma lista longa por engano não abrir meia
   faixa de portas no firewall.
3. **Rótulo opcional por porta** ("console", "métricas"), para a tela de Ingress e a aba Conexão
   ficarem legíveis. Não é necessário; custa um campo a mais.
