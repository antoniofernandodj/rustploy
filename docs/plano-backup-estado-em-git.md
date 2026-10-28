# Plano: backup do estado em git (espelho versionado, com segredos cifrados)

**Estado:** **planejado, nada implementado.** Escrito em 2026-10-26.
**Sessão de chat:** a conversa que projetou este plano pode ser retomada com `claude --resume 9f61d38c-7929-4171-a58e-17c9f2fe6fb9`.

---

## 1. O problema, em linguagem simples

Hoje todo o estado declarado do servidor (projetos, serviços, variáveis de ambiente, segredos,
provedores git, bancos gerenciados) mora em **um arquivo SQLite no disco do próprio servidor**.
Se o servidor se perde (disco morre, VPS apagada), perde-se tudo junto.

O que já existe protege contra acidente, **não** contra perda do servidor:

- `features/backup` grava snapshots JSON das env vars a cada 60 s, mas **no mesmo disco**
  (`<db_path>/env_backups/`). Ver [env-backup.md](env-backup.md).
- `ManifestExportAll` exporta tudo num `rustploy.yml` + `rustploy.vars.toml`, mas **só quando
  alguém clica** na tela de Settings.

O objetivo: **a cada mudança, o daemon grava o estado num repositório git privado no GitHub**,
de modo que dê para (a) ver o histórico de quem mudou o quê e (b) reconstruir um servidor
novo a partir dele, **inclusive os segredos**.

## 2. A decisão de arquitetura (e o que ficou de fora)

**O SQLite continua sendo a fonte da verdade.** O git é um **espelho de mão única**: só o
daemon escreve nele; ninguém edita o YAML à mão.

Por que não o contrário (YAML como fonte, SQLite só para runtime)? Foi analisado e descartado
nesta sessão: a GUI e a webui teriam de reescrever o arquivo a cada clique (comentários e
ordem se perdem, working tree sujo, dois escritores no mesmo arquivo), parte do estado nasce
em runtime (portas, tokens de webhook), e a troca mexe na camada `store` inteira. O ganho
(git como fonte) não é o que se quer aqui; o que se quer é **versionar e recuperar**.
Com espelho de mão única somem de graça os problemas de drift, de `managed_by: git` e de
conflito de merge.

Também descartado: um YAML único gigante (conflito, um erro derruba tudo, diff ilegível) e
versionar o `.db` em git (binário, sem diff, sem merge).

**Um YAML por projeto**, mais um arquivo raiz que os agrega, é o formato (o manifesto atual
já suporta `include:`).

## 3. O que já existe e será reaproveitado

| Peça | Onde | Para quê |
|---|---|---|
| `ManifestExportAll` | `features/iac/manifest_export_all.rs` | gera o manifesto raiz; já redige env `Plain` para `${KEY}` e separa os valores reais num TOML (`EnvDoc`) |
| `ManifestImport` | `features/iac/manifest_import.rs` | restaura, reconciliando por nome |
| `SecretsManager` | `platform/secrets.rs` | cifra `age` (passphrase) em hex; chave em `/etc/rustploy/master.key` |
| `features/backup` | `features/backup/mod.rs` | tarefa periódica com `interval`; modelo para o novo gatilho |
| `git_provider` / `webhook_token` | `features/git_providers/`, `store/` | infraestrutura de credenciais git |

## 4. Segredos: o ponto crítico

### 4.1 Onde cada dado sensível está hoje (verificado no código)

| Dado | Como está guardado | Situação |
|---|---|---|
| `secret.value` (segredos de projeto) | cifrado (`age`, hex) | pode ir ao git como está |
| `git_provider`: `oauth_client_secret_enc`, `access_token_enc`, `refresh_token_enc` | cifrado | idem |
| `registry_tokens.token_sha256` | hash | idem (mas o token real não é recuperável) |
| **env `Plain` dos serviços** (`service.spec`, JSON) | **texto puro** | o export atual os tira do YAML e põe no `rustploy.vars.toml` **em claro** |
| **`managed_database.password`** | **texto puro** | o export atual **nem inclui** bancos gerenciados |
| **`webhook_token.token`** | **texto puro** | não é exportado |
| `master.key` | arquivo em `/etc/rustploy/` | o próprio segredo mestre |

Consequência: o `rustploy.vars.toml` que o export já produz **não pode ir ao git do jeito que
está**, porque carrega valores reais em claro. E os bancos gerenciados precisam entrar no
backup, senão uma restauração perde as senhas dos bancos.

### 4.2 A regra

> **Nada vai ao git sem passar por `age`.** O exportador trata todo campo como sensível por
> padrão e deixa em claro **só uma lista explícita de campos seguros** (nome, imagem, porta,
> domínio, replicas…). Campo novo adicionado no futuro nunca vaza por esquecimento.

### 4.3 Dois cofres, duas chaves

- **No repo:** todo conteúdo sensível vai num arquivo `*.secrets.age` por projeto, cifrado
  para uma **chave pública de recuperação** (`age` com *recipient*, não passphrase).
- **A chave privada correspondente** fica **fora do servidor e fora do git** (gerenciador de
  senhas, cofre offline). Só você a tem.
- A `master.key` de produção **não precisa sair do servidor**: o daemon decifra com ela, e
  recifra para a chave pública de recuperação na hora de exportar.

Assim, vazar o repo não revela nada, perder o servidor não perde nada, e a chave
de recuperação é **um item pequeno que você guarda uma vez**.

Alternativa mais simples (pior): commitar o ciphertext que já está no banco e guardar a
`master.key` à parte. Funciona, mas obriga a levar a `master.key` do servidor (que muda de
dono/máquina) e amarra o backup à rotação dela.

### 4.4 Cuidados já identificados

- `SecretsManager::new` cria a `master.key` com `std::fs::write` (permissão padrão, em geral
  0644). Restringir a **0600** na criação e corrigir as já existentes. Independe deste plano,
  mas ele aumenta a importância do arquivo.
- O token/deploy key para dar `push` é mais um segredo: guardá-lo cifrado como os demais.
- Repositório **privado** sempre: mesmo sem segredos, o YAML expõe topologia, domínios e imagens.
- **Rotação da `master.key`** recifra o banco, mas o repo está cifrado para a chave de
  recuperação, não para a `master.key`, então a rotação não invalida o histórico. É uma
  vantagem da §4.3.

## 5. Formato do repositório

```
<repo>/
├── stack.yml                  # raiz: lista e `include:` dos projetos
├── projects/
│   ├── <projeto>/
│   │   ├── rustploy.yml       # serviços, domínios, jobs (sem valores sensíveis)
│   │   └── secrets.age        # envs Plain + secrets + senhas de bancos (cifrado)
├── providers.yml              # git providers (sem tokens)
├── providers.secrets.age      # tokens/segredos OAuth
└── MANIFEST.json              # versão do daemon, versão do schema, data, hash do estado
```

Regras do conteúdo:

- **Serialização determinística**: projetos e serviços ordenados por nome, chaves em ordem
  estável (`BTreeMap` já é usado no export). Sem isso, todo export gera diff de ruído.
- O `id` de projeto/serviço entra no YAML. É isso que preserva a identidade na restauração
  (o histórico de deploys aponta para IDs, não para nomes).
- Nada de runtime: `deployment`, `build_log`, `job_run`, `registry_*`, `tls_cert` ficam de fora.
- **`secrets.age` precisa ser estável**: `age` gera salt/nonce novos a cada cifragem, então
  recifrar o mesmo conteúdo produz bytes diferentes e um commit a cada ciclo. Solução: só
  recifrar quando o **conteúdo em claro** mudou (comparar o hash do plaintext, guardado em
  `MANIFEST.json`, nunca o ciphertext).

## 6. Gatilho e fluxo

1. Algo declarativo muda (criar/editar/apagar projeto, serviço, segredo, provider, banco).
   O daemon já tem um barramento de eventos em `platform/`; publicar um evento
   `StateChanged` ali (ou observar os eventos existentes).
2. **Debounce** de alguns segundos (várias edições seguidas viram um commit só).
3. Gerar o export em um diretório temporário, comparar com o checkout local do repo.
   **Sem diff, sem commit.**
4. Commit com mensagem útil (`service api: env alterado`) e `push`.
5. Falha de push (rede, credencial) **não atrapalha o daemon**: loga, mantém o commit local
   e tenta de novo no próximo ciclo. O estado nunca bloqueia por causa do backup.

Configuração em `config.toml` (parse é tudo-ou-nada; campo novo precisa de default):

```toml
[state_git]
enabled = false                 # desligado por padrão
remote  = "git@github.com:org/rustploy-state.git"
branch  = "main"
debounce_secs = 10
recovery_recipient = "age1..."  # chave PÚBLICA de recuperação
```

## 7. Restauração

1. Instalar o rustploy no servidor novo.
2. Clonar o repo, fornecer a **chave privada de recuperação**.
3. `rustploy restore <repo>` (ou botão em Settings): decifra `*.secrets.age`, importa o YAML
   com os IDs preservados, grava os segredos **recifrando com a `master.key` nova**.
4. Serviços voltam como `Stopped`; um redeploy (ou o autostart, quando existir) os sobe.

O `ManifestImport` atual reconcilia por nome e **gera IDs novos**. Preservar os IDs é um
trabalho novo (modo `restore` separado do `apply`, para não mudar o comportamento atual).

## 8. Fases

| Fase | Entrega | Risco |
|---|---|---|
| **0** | Fechar o que falta saber (ver §10) e restringir a `master.key` a 0600 | baixo |
| **1** | Exportador determinístico por projeto + `*.secrets.age` (sem git ainda): `Command::StateExport` que grava num diretório. Teste: exporta duas vezes, diff vazio | médio |
| **2** | Restauração com IDs preservados (`StateRestore`) e teste de ida e volta: exporta, apaga o banco, restaura, compara | médio-alto |
| **3** | Gatilho com debounce + commit/push (usar `git` do sistema ou `git2`; **sem reqwest**, ver memória) + `[state_git]` na config | médio |
| **4** | UI: Settings na GUI **e** na webui (estado, último commit, botão "exportar agora", botão "restaurar"), conforme a regra dos dois clientes | baixo |
| **5** | Docs (`rustploy-docs`, pt e en) e checklist de "Publish" | baixo |

A ordem importa: **sem a Fase 2 provada, o backup é uma promessa não testada.** Só considerar
a feature pronta depois de restaurar de verdade num servidor limpo.

## 9. Testes

- Determinismo: dois exports seguidos do mesmo estado geram bytes idênticos (menos `*.age`,
  que se compara pelo plaintext).
- **Vazamento**: teste que percorre o diretório exportado e falha se qualquer valor de env,
  senha ou token conhecido do banco de teste aparecer em claro em qualquer arquivo.
- Ida e volta: exporta, apaga o banco, restaura, e o `ManifestExportAll` do resultado é igual
  ao original.
- Chave errada: restaurar com a chave de recuperação errada falha sem escrever nada.
- Push com falha não derruba nem bloqueia o daemon.
- Por ser feature de UI também: rodar a GUI e a webui de verdade antes de dar por pronto.

## 10. Pontos em aberto

- ~~`service.env` e `managed_database.password` estão em claro?~~ **Confirmado:** ambos em texto
  puro (env no JSON de `service.spec`; a senha vai direto do banco para a view em
  `databases/managed_view.rs`).
- `git` do sistema ou crate `git2`/`gix`? O `git` do sistema é mais simples, mas vira dependência
  de runtime do pacote `.deb`. Decidir na Fase 3.
- **Mais algum segredo não passa pelo cofre?** Falta varrer `daemon_settings` (pode guardar
  credenciais), as credenciais de ACME e o token da API, que está no `config.toml`, não no banco.
  O token da API e a `master.key` ficam fora deste backup (são do servidor, não do estado); a
  restauração gera os novos.
- Os blobs do registry embutido e os volumes dos containers **não** estão no escopo: este
  plano cobre configuração, não dados. Backup de dados (bancos, volumes) é outro assunto.
- Estratégia de retenção: o histórico git cresce a cada mudança. Provavelmente aceitável (texto
  pequeno), mas vale medir antes de prometer.
