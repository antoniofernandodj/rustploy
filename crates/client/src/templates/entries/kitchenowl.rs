use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kitchenowl",
    name: "KitchenOwl",
    description: "Organizador inteligente de receitas e listas de supermercado",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  kitchenowl:
    image: tombursch/kitchenowl:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      JWT_SECRET_KEY: {{JWT_SECRET_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "JWT_SECRET_KEY",
        label: "JWT Secret Key",
        default: None,
        required: true,
        secret: true,
    }],
};
