use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "xsshunter",
    name: "XSSHunter",
    description: "Ferramenta para pesquisadores focada em Blind XSS",
    category: TemplateCategory::Security,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: xsshunter
      POSTGRES_USER: xsshunter
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  xsshunter:
    image: thehackerish/xsshunter-client:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      DATABASE_URL: postgresql://xsshunter:{{DB_PASSWORD}}@db:5432/xsshunter
      SECRET: {{SECRET}}
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
            key: "SECRET",
            label: "Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
