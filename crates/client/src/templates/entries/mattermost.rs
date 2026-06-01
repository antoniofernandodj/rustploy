use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "mattermost",
    name: "Mattermost",
    description: "Chat corporativo focado em DevOps (Postgres)",
    category: TemplateCategory::Communication,
    default_port: 8065,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: mattermost
      POSTGRES_USER: mattermost
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  mattermost:
    image: mattermost/mattermost-team-edition:latest
    restart: unless-stopped
    ports:
      - "8065"
    environment:
      MM_SQLSETTINGS_DRIVERNAME: postgres
      MM_SQLSETTINGS_DATASOURCE: postgres://mattermost:{{DB_PASSWORD}}@db:5432/mattermost?sslmode=disable
      MM_SERVICESETTINGS_SITEURL: http://{{DOMAIN}}
    volumes:
      - mm_data:/mattermost/data
    depends_on:
      - db

volumes:
  db_data:
  mm_data:
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
            key: "DOMAIN",
            label: "URL do site",
            default: Some("localhost:8065"),
            required: true,
            secret: false,
        },
    ],
};
