use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "ackee",
    name: "Ackee",
    description: "Analytics focado em privacidade para websites",
    category: TemplateCategory::Analytics,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - db_data:/data/db

  ackee:
    image: electerious/ackee:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      MONGODB: mongodb://db:27017/ackee
      ACKEE_USERNAME: {{ADMIN_USER}}
      ACKEE_PASSWORD: {{ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
    variables: &[
        TemplateVar {
            key: "ADMIN_USER",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
