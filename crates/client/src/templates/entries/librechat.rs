use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "librechat",
    name: "LibreChat",
    description: "Interface unificada para múltiplos provedores de IA",
    category: TemplateCategory::Ai,
    default_port: 3080,
    compose: r#"
services:
  db:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - db_data:/data/db
  librechat:
    image: ghcr.io/danny-avila/librechat:latest
    restart: unless-stopped
    ports:
      - "3080"
    environment:
      MONGO_URL: mongodb://db:27017/librechat
      JWT_SECRET: {{JWT_SECRET}}
      JWT_REFRESH_SECRET: {{JWT_REFRESH_SECRET}}
    volumes:
      - images:/app/client/public/images
    depends_on:
      - db

volumes:
  db_data:
  images:
"#,
    variables: &[
        TemplateVar {
            key: "JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "JWT_REFRESH_SECRET",
            label: "JWT Refresh Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
