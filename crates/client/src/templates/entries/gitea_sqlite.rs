use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "gitea-sqlite",
    name: "Gitea",
    description: "Servidor Git leve e auto-hospedado (SQLite)",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  gitea:
    image: gitea/gitea:latest
    restart: unless-stopped
    expose:
      - "3000"
      - "22"
    environment:
      USER_UID: 1000
      USER_GID: 1000
    volumes:
      - gitea_data:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro

volumes:
  gitea_data:
"#,
    variables: &[],
};
