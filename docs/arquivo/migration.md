# Migração de Outras Plataformas

Rustploy suporta a importação de dados de outras plataformas PaaS para facilitar a transição. Atualmente, o suporte está focado no **Dokploy**.

## Importando do Dokploy

O comando `import` permite extrair projetos, aplicações e configurações de Docker Compose diretamente do banco de dados do Dokploy.

### Pré-requisitos

1. O banco de dados do Dokploy (PostgreSQL) deve estar acessível.
2. O daemon `rustployd` deve estar instalado e com o banco de dados inicializado em `/var/lib/rustploy/db/rustploy.db`.

### Como Usar

Para realizar uma importação, utilize o comando `import` no binário `rustploy` (cliente):

```bash
# Dry-run para verificar o que será importado sem realizar alterações
rustploy import dokploy --pg-url "postgresql://user:password@localhost:5432/dokploy" --dry-run

# Executar a migração real
sudo -u rustploy rustploy import dokploy --pg-url "postgresql://user:password@localhost:5432/dokploy" --yes

# Gerar um arquivo SQL com os comandos de migração (para execução manual)
rustploy import dokploy --pg-url "postgresql://user:password@localhost:5432/dokploy" --output-sql migration.sql
```

### O que é migrado

- **Projetos:** Todos os projetos são criados com novos IDs (ULID).
- **Aplicações:** Convertidas para serviços do tipo Git.
- **Docker Compose:** Migrados integralmente. Redes específicas do Dokploy são limpas automaticamente.
- **Domínios e Portas:** Mapeados para o sistema de ingress do Rustploy.
- **Variáveis de Ambiente:** Migradas como valores planos (Plain).

### O que não é migrado (Ações Manuais Necessárias)

- **Segredos:** Variáveis marcadas como segredos no Dokploy devem ser reconfiguradas no Rustploy para garantir a criptografia correta.
- **Volumes:** Mapeamentos de volumes complexos podem precisar de revisão manual no spec do serviço.
- **Histórico de Deployments:** Apenas as definições atuais são migradas.
- **Credenciais Git:** Você precisará reconfigurar as chaves SSH ou tokens se os repositórios forem privados.

## Solução de Problemas

- **Permissão Negada no SQLite:** Se o comando falhar ao abrir o banco de dados do Rustploy, certifique-se de rodar o comando com o usuário `rustploy` ou com permissões adequadas de escrita em `/var/lib/rustploy/db/`.
- **Conexão com Postgres:** Certifique-se de que o container do Postgres do Dokploy tem a porta exposta ou que você está rodando o import de dentro da mesma rede Docker.
