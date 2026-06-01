use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "hoarder",
    name: "Hoarder",
    description: "Bookmarks inteligentes com auto-tagging baseado em IA",
    category: TemplateCategory::Ai,
    default_port: 3000,
    compose: r#"
services:
  hoarder:
    image: ghcr.io/hoarder-app/hoarder:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      MEILI_MASTER_KEY: {{MEILI_MASTER_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "NEXTAUTH_SECRET",
            label: "NextAuth Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "MEILI_MASTER_KEY",
            label: "Meilisearch Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
