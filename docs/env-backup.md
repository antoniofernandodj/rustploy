# Backup automático de env vars

O daemon grava automaticamente um snapshot de **todas as env vars** (projetos e serviços) a cada 60 segundos, sem sobrescrever ficheiros anteriores. Isso protege contra perdas acidentais — por exemplo, ao aplicar um manifesto exportado sem vars.

---

## Localização dos snapshots

Por defeito os ficheiros ficam em:

```
<db_path>/env_backups/
# normalmente: /var/lib/rustploy/db/env_backups/
```

Cada ficheiro tem o nome:

```
env_backup_2026-06-26T00-31-29Z.json
```

---

## Configuração (`/etc/rustploy/config.toml`)

```toml
[env_backup]
# Directório onde os snapshots são gravados (opcional — default: <db_path>/env_backups/)
dir = "/var/lib/rustploy/db/env_backups"

# Intervalo entre snapshots em segundos (default: 60)
interval_secs = 60
```

O bloco `[env_backup]` é opcional. Se omitido, os valores acima são usados.

---

## Comandos

### Listar snapshots disponíveis

```bash
rustploy env-backup list
```

Exemplo de saída:

```
Snapshots disponíveis (3):
  env_backup_2026-06-26T01-00-00Z.json
  env_backup_2026-06-26T00-59-00Z.json
  env_backup_2026-06-26T00-58-00Z.json
```

### Restaurar um snapshot

```bash
rustploy env-backup restore env_backup_2026-06-26T00-58-00Z.json
```

A restauração:
- Actualiza as env vars de cada projeto e serviço cujo ID ainda existe na DB
- **Não apaga** projetos nem serviços — só actualiza as vars
- Após restaurar, redeploya os serviços afectados para aplicar as novas vars

---

## Limpeza automática

No início de cada mês o daemon apaga automaticamente os snapshots com **mais de 31 dias**, mantendo todo o histórico do mês corrente e do anterior.

---

## Formato do snapshot (JSON)

```json
{
  "created_at": "2026-06-26T00:58:00Z",
  "projects": [
    {
      "id": "prj_01KV...",
      "name": "Chiquitos",
      "env_vars": [
        { "key": "POSTGRES_USER", "value": { "Plain": "myuser" } },
        { "key": "DATABASE_URL",  "value": { "Plain": "postgres://..." } }
      ]
    }
  ],
  "services": [
    {
      "id": "svc_01KV...",
      "name": "api",
      "project_id": "prj_01KV...",
      "env_vars": []
    }
  ]
}
```

Secrets armazenados como `EnvVarValue::Secret` (referência a um nome de secret) são gravados **como referência**, não como valor em claro — o valor cifrado permanece no gestor de secrets.

---

## Porquê foi criado

O comando `rustploy apply` aplicava manifestos exportados (sem env vars) e, ao detectar diferença com o estado existente, sobrescrevia as vars com uma lista vazia. O fix para esse bug (v ea670d9) está em vigor, mas o backup contínuo garante uma segunda linha de defesa independente da lógica de apply.
