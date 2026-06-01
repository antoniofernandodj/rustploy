use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "listmonk",
    name: "Listmonk",
    description: "Gerenciador de newsletters e e-mail marketing (Postgres)",
    category: TemplateCategory::Automation,
    default_port: 9000,
    compose: r#"
services:
  db:
    image: postgres:13
    restart: unless-stopped
    environment:
      POSTGRES_DB: listmonk
      POSTGRES_USER: listmonk
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  listmonk:
    image: listmonk/listmonk:latest
    restart: unless-stopped
    ports:
      - "9000"
    environment:
      LISTMONK_db__host: db
      LISTMONK_db__port: "5432"
      LISTMONK_db__user: listmonk
      LISTMONK_db__password: {{DB_PASSWORD}}
      LISTMONK_db__database: listmonk
    depends_on:
      - db

volumes:
  db_data:
"#,
    variables: &[TemplateVar {
        key: "DB_PASSWORD",
        label: "Senha do banco",
        default: None,
        required: true,
        secret: true,
    }],
};
