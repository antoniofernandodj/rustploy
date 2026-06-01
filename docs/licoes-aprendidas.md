# Lições aprendidas — erros, causas e soluções

Registro das dificuldades reais encontradas durante o desenvolvimento do Rustploy.

---

## 1. Docker: alias de rede não funciona entre redes diferentes

### Contexto

O serviço Compose do Postgres declarava um alias `db` na rede `default` do Compose:

```yaml
services:

  postgres:
    networks:
      default:
        aliases:
          - db
```

A ideia era que o serviço Application (`hw`) usasse `db` como hostname na `DATABASE_URL`.

### O erro

O container da API subia, tentava conectar em `db:5432` no `ensure_table()` e morria com exit code 1. O deploy falhava no healthcheck.

### Causa

O Docker DNS só resolve aliases **dentro da rede em que eles foram definidos**. O alias `db` foi declarado na rede interna do Compose (`rp_postgres_default`). O serviço Application está em outra rede (`rp_net_<project_id>`). Mesmo que o container do Postgres esteja conectado às duas redes, o alias não "vaza" para a outra rede — ele pertence exclusivamente àquela onde foi configurado.

```
rp_postgres_default:  postgres → alias "db" ✓
rp_net_01KSZAYK:      postgres → sem alias  ✗  (apenas o nome do container)
```

### Solução

Usar o **nome do container** como hostname, que o Docker DNS resolve em qualquer rede que o container compartilhe:

```
DATABASE_URL=postgresql://appuser:secret@rp_postgres-postgres-1:5432/myapp
```

O padrão de nomes dos containers Compose no Rustploy é:

```
rp_{nome_do_service_compose}-{nome_do_serviço_no_yaml}-1
```

### Lição

Aliases de rede no Docker Compose são locais à rede onde são definidos. Para comunicação entre redes distintas, use sempre o nome do container — que é resolvível via Docker DNS em qualquer rede compartilhada.

---

## 2. Python: crash silencioso no startup sem retry

### Contexto

A API Python chamava `ensure_table()` na inicialização (antes de subir o servidor HTTP), que abria conexão com o Postgres.

### O erro

O container subia, falhava silenciosamente, e o Rustploy detectava exit code 1 durante o healthcheck polling — sem mensagem de erro clara nos logs do deploy.

### Causa

Duas causas combinadas:

1. O hostname `db` não resolvia (problema anterior).
2. Não havia retry — qualquer falha de conexão no startup derrubava o processo imediatamente.

Isso é especialmente problemático em ambientes de container onde o banco pode não estar pronto quando a API sobe, mesmo que já esteja "running".

### Solução

```python
def ensure_table():
    for attempt in range(1, 11):
        try:
            with get_conn() as conn:
                # DDL aqui
            return
        except Exception as e:
            print(f"DB not ready (attempt {attempt}/10): {e}")
            time.sleep(3)
    raise RuntimeError("Could not connect to database after 10 attempts")
```

### Lição

Nunca conectar a dependências externas no startup sem retry. Containers sobem em paralelo e o banco quase sempre estará alguns segundos atrás da aplicação. O padrão correto é: **tentar N vezes com intervalo**, só falhar definitivamente após esgotar as tentativas.

---

## 3. cargo-deb: caminhos de assets relativos ao crate, não ao workspace

### Contexto

O Rustploy é um workspace Cargo com três crates. Os arquivos de packaging ficam na raiz do workspace (`packaging/`). A configuração do `cargo-deb` fica no `Cargo.toml` do crate `daemon` (`crates/daemon/Cargo.toml`).

### O erro

```
error: Can't resolve asset: crates/daemon/packaging/config.toml -> etc/rustploy/config.toml
  cause: Static file asset path did not match any existing files:
         /home/.../rustploy/crates/daemon/packaging/config.toml
```

### Causa

O `cargo-deb` resolve caminhos de assets **relativos ao `Cargo.toml` do crate**, não à raiz do workspace. Como o crate está em `crates/daemon/`, o path `packaging/config.toml` era interpretado como `crates/daemon/packaging/config.toml`.

### Solução

Usar `../..` para subir até a raiz do workspace:

```toml
[package.metadata.deb]
assets = [
    ["../../packaging/config.toml",       "etc/rustploy/config.toml",            "644"],
    ["../../packaging/rustployd.service", "lib/systemd/system/rustployd.service", "644"],
]
```

### Lição

Em workspaces Cargo, o `cargo-deb` (e outras ferramentas similares) sempre resolvem paths relativos ao `Cargo.toml` do crate, não ao workspace. Use `../../` para referenciar arquivos na raiz do workspace.

---

## 4. cargo-deb: campos inválidos em `[package.metadata.deb]`

### Contexto

Ao tentar suprimir warnings do `cargo-deb` sobre `license` e `description` ausentes, os campos foram adicionados diretamente ao `[package.metadata.deb]`.

### O erro

```
error: Unable to parse Cargo.toml
  cause: unknown field `description`, expected one of `name`, `maintainer`, ...
error: Unable to parse Cargo.toml
  cause: unknown field `license`, expected one of `name`, `maintainer`, ...
```

### Causa

O `[package.metadata.deb]` tem seu próprio schema fixo. Os campos `description` e `license` **não existem** nele:

- O equivalente a `description` é `extended-description`
- O equivalente a `license` é `license-file` (aponta para um arquivo, não uma string)

Os warnings sobre `license` e `description` ausentes vêm do `[package]` do Cargo, não do `[package.metadata.deb]`. A solução correta é adicionar ao bloco `[package]`:

```toml
[package]
name = "daemon"
version = "0.1.0"
license = "MIT"
description = "Rustploy PaaS daemon"
```

### Lição

`[package.metadata.deb]` e `[package]` são seções separadas com schemas independentes. Warnings do `cargo-deb` sobre campos ausentes geralmente se referem ao `[package]`, não ao `[package.metadata.deb]`. Consulte a lista de campos válidos antes de adicionar qualquer um.

Campos válidos em `[package.metadata.deb]` (cargo-deb 3.x): `name`, `maintainer`, `copyright`, `license-file`, `changelog`, `depends`, `pre-depends`, `recommends`, `suggests`, `enhances`, `conflicts`, `breaks`, `replaces`, `provides`, `extended-description`, `extended-description-file`, `section`, `priority`, `revision`, `conf-files`, `assets`, `maintainer-scripts`, `systemd-units`, entre outros.

---

## 5. Makefile: variável avaliada antes do arquivo existir

### Contexto

O Makefile definia o caminho dos `.deb` como variável no topo:

```makefile
DAEMON_DEB := target/debian/rustployd_$(VERSION)_$(ARCH).deb
```

### O erro

```
dpkg: error: cannot access archive 'target/debian/rustployd_0.1.0_amd64.deb': No such file or directory
```

O arquivo real gerado era `rustployd_0.1.0-1_amd64.deb` (com revisão `-1` adicionada pelo `cargo-deb`).

### Causa

Dois problemas combinados:

1. **O nome do arquivo estava errado**: o `cargo-deb` adiciona sufixo de revisão `-1` por padrão, resultando em `0.1.0-1` no nome, não `0.1.0`.
2. **Variáveis no topo do Makefile são avaliadas em tempo de parse**, antes de qualquer target rodar. Mesmo corrigindo o nome, o `$(shell ls ...)` no topo seria avaliado antes do `make deb` gerar os arquivos.

### Solução

Usar `$$()` (shell expansion em tempo de execução) diretamente nos targets, com glob para não depender do sufixo exato:

```makefile
.PHONY: install
install: deb
	sudo dpkg -i $$(ls target/debian/rustployd_*.deb | tail -1)
	sudo dpkg -i $$(ls target/debian/rustploy_*.deb  | tail -1)
```

O `$$()` em um target Makefile é passado ao shell como `$()` — avaliado quando o target executa, não quando o Makefile é parseado.

### Lição

- Variáveis no topo do Makefile (`:=` e `=`) são avaliadas cedo. Nunca use `$(shell ...)` no topo para encontrar arquivos que ainda não existem.
- Use `$$()` dentro de receitas de targets para avaliação tardia (em tempo de execução).
- Prefira globs (`rustployd_*.deb`) a nomes exatos quando ferramentas externas podem adicionar sufixos ao filename.
