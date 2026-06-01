use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "phpmyadmin",
    name: "phpMyAdmin",
    description: "Gerenciador web para bancos MySQL e MariaDB",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  phpmyadmin:
    image: phpmyadmin:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      PMA_HOST: {{PMA_HOST}}
      MYSQL_ROOT_PASSWORD: {{MYSQL_ROOT_PASSWORD}}
"#,
    variables: &[
        TemplateVar {
            key: "PMA_HOST",
            label: "Host MySQL",
            default: Some("db"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "MYSQL_ROOT_PASSWORD",
            label: "Senha root MySQL",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
