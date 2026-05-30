# Como funciona o ingress proxy e o zero-downtime deploy

## O problema com port bindings diretos

Quando um container Docker é criado com uma porta publicada no host (ex: `-p 80:3000`),
esse binding fica **preso ao container** até ele ser removido. Dois containers não podem
publicar a mesma porta ao mesmo tempo — o Docker rejeita com erro 500.

Isso torna zero-downtime impossível com port bindings: para trocar o container live pelo
novo, você teria que parar o velho primeiro, criando uma janela de indisponibilidade.

---

## A solução: ingress proxy

O Rustploy resolve isso com um **proxy reverso interno** que fica na porta do host.
Os containers ficam na **rede Docker interna** e nunca publicam porta no host.

```
Navegador
    │
    │  http://localhost:8080/ok
    ▼
┌─────────────────────────────┐
│  ingress proxy (porta 8080) │  ← único processo na porta do host
│  tabela de rotas:           │
│    localhost → 172.23.0.2:3000
└──────────────┬──────────────┘
               │  repassa a requisição
               ▼
┌─────────────────────────────┐
│  container rp_teste         │
│  IP: 172.23.0.2             │
│  porta interna: 3000        │
└─────────────────────────────┘
```

A tabela de rotas é atualizada **atomicamente** quando um deploy conclui. O proxy
nunca para — só troca para onde aponta.

---

## Fluxo completo: do deploy ao Live

### Estados da máquina de deploy

```
Pending
  └─► ResolvingDeps
        └─► CloningRepo ──► BuildingImage ──► Staging ──► HealthcheckPolling
                                                                  │
                                                             (passou?)
                                                                  │
                                                            SwappingIn ──► Promoting ──► Live
                                                                  │
                                                           (falhou?)
                                                                  │
                                                            RollingBack ──► Failed
```

### Passo a passo

#### 1. Pending
O executor garante que a rede Docker do projeto existe.
Cada projeto tem uma rede bridge isolada chamada `rp_net_<id[:8]>`.

#### 2. ResolvingDeps
Decide o próximo estado com base na fonte do serviço:
- Fonte Git → vai para `CloningRepo`
- Fonte Registry → vai para `PullingImage` (direto para staging)

#### 3. CloningRepo
Clona o repositório Git para um diretório temporário em `<db_path>/builds/<deploy_id>/`.
Nenhum container existe ainda.

#### 4. BuildingImage
Constrói a imagem Docker a partir do `Dockerfile` no repositório clonado.
A imagem recebe a tag `rp_<nome_do_serviço>:<deploy_id[:8]>`.

Os logs do build são publicados como eventos `LogLine` e aparecem na aba Logs do TUI.

#### 5. Staging
Cria o container de staging com o nome `rp_<serviço>_staging_<deploy_id[:8]>`.

```
Criação:   rp_teste_staging_01KSTW90
Rede:      rp_net_01KSTW2R  (rede interna do projeto)
Porta:     3000/tcp (exposta internamente, SEM binding no host)
```

O container é criado, conectado à rede do projeto e então iniciado.
Nesse ponto o live antigo continua rodando normalmente — nenhuma interrupção.

#### 6. HealthcheckPolling
O executor obtém o IP interno do container de staging na rede do projeto
e testa se a aplicação está respondendo.

Exemplo de healthcheck HTTP:
```
GET http://172.23.0.5:3000/ok  →  200 OK  ✓
```

O polling respeita:
- `start_period`: aguarda N segundos antes do primeiro check
- `interval`: intervalo entre tentativas
- `timeout`: tempo máximo por tentativa
- `retries`: número máximo de tentativas antes de falhar

Se o healthcheck passar → `SwappingIn`.
Se esgotar as tentativas → `RollingBack`.

#### 7. SwappingIn
Este é o passo crítico do zero-downtime.

1. Obtém o IP do container de staging.
2. **Se o serviço tem domínio configurado**: atualiza a tabela de rotas do proxy
   para apontar para o novo container — a mudança é atômica (arc-swap).
3. Para o container live antigo (se existir).

```
Antes:  localhost → 172.23.0.2:3000  (container antigo)
Depois: localhost → 172.23.0.5:3000  (container novo)
```

A janela entre "atualizar rota" e "parar container antigo" é de milissegundos.
Requisições em voo no container antigo terminam normalmente (drain period).

#### 8. Promoting
1. Remove o container live antigo.
2. Renomeia o staging para o nome live: `rp_teste_staging_X` → `rp_teste`.
3. Atualiza o status do serviço para `Running`.
4. Remove o diretório de build temporário.

#### 9. Live ✓
O serviço está rodando. O proxy aponta para o novo container.

---

## Redeploy (segundo deploy em cima de um serviço live)

O fluxo é idêntico ao primeiro deploy, com uma diferença: no passo `SwappingIn`
existe um container live anterior.

```
TIMELINE:

t=0   Deploy iniciado. Live antigo continua servindo 100% do tráfego.
      [rp_teste] → proxy → usuários

t=1   Staging criado. Live continua rodando em paralelo.
      [rp_teste]         → proxy → usuários
      [rp_teste_staging] → sem tráfego (só healthcheck interno)

t=2   Healthcheck passou. SwappingIn:
      proxy atualizado atomicamente → aponta para staging
      [rp_teste]         → proxy → usuários  (ainda vivo por milissegundos)
      [rp_teste_staging] → proxy → usuários  (assumindo tráfego)

t=3   Live antigo parado. Staging renomeado para live.
      [rp_teste] → proxy → usuários  (novo container, nome antigo)

Downtime: 0ms
```

---

## Configuração necessária

### 1. Porta do proxy

O proxy escuta na porta `8080` por padrão (sem necessidade de root).
Para usar a porta `80`:

```bash
# via env var
RUSTPLOY_HTTP_PORT=80 cargo run -p daemon

# ou via ~/.config/rustploy/config.toml
[ingress]
http_port = 80
```

### 2. Domínio do serviço

No TUI, aba **Domains** do serviço:

| Campo         | Valor para dev local |
|---------------|----------------------|
| Domínio       | `localhost`          |
| Porta externa | (deixar vazio)       |

O proxy roteará qualquer requisição com header `Host: localhost` para este serviço.

### 3. Acessar o serviço

```bash
curl http://localhost:8080/ok
# ou no navegador: http://localhost:8080/ok
```

### Múltiplos serviços no mesmo host

Cada serviço recebe um domínio diferente:

```
serviço-a → domínio: api.meuapp.com
serviço-b → domínio: web.meuapp.com
```

Todos são acessados pelo mesmo proxy na porta 8080 (ou 80).
O roteamento é feito pelo header `Host` da requisição.

---

## Por que não usar port binding direto?

| Característica     | Port binding direto | Ingress proxy       |
|--------------------|--------------------|--------------------|
| Zero downtime      | ✗ Impossível       | ✓                  |
| Múltiplos serviços | Porta diferente p/ cada um | Uma porta p/ todos |
| Redeploy           | Erro 500 Docker    | Transparente       |
| Acesso             | `localhost:3000`   | `localhost:8080`   |
| Configuração       | Automático         | Requer domínio     |
