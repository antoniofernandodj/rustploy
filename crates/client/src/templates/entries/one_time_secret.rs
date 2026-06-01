use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "one-time-secret",
    name: "One Time Secret",
    description: "Compartilhamento seguro de segredos por links que se destroem",
    category: TemplateCategory::Security,
    default_port: 7143,
    compose: r#"
services:
  redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - redis_data:/data
  one-time-secret:
    image: onetimesecret/onetimesecret:latest
    restart: unless-stopped
    ports:
      - "7143"
    environment:
      REDIS_URL: redis://redis:6379
      OTS_SECRET: {{OTS_SECRET}}
    depends_on:
      - redis

volumes:
  redis_data:
"#,
    variables: &[TemplateVar {
        key: "OTS_SECRET",
        label: "Secret",
        default: None,
        required: true,
        secret: true,
    }],
};
