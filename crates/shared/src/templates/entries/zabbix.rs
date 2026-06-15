use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "zabbix",
    name: "Zabbix",
    description: "Monitor corporativo robusto para redes e servidores",
    category: TemplateCategory::Monitoring,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zabbix
      POSTGRES_USER: zabbix
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zabbix:
    image: zabbix/zabbix-web-nginx-pgsql:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      DATABASE_URL: postgresql://zabbix:{{DB_PASSWORD}}@db:5432/zabbix
      ZBX_SERVER_HOST: {{ZBX_SERVER_HOST}}
    depends_on:
      - db

volumes:
  db_data:
"#,
    variables: &[
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "ZBX_SERVER_HOST",
            label: "Host do Zabbix Server",
            default: Some("zabbix-server"),
            required: true,
            secret: false,
        },
    ],
};
