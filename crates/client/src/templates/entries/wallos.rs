use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "wallos",
    name: "Wallos",
    description: "Rastreador pessoal de assinaturas mensais e gastos recorrentes",
    category: TemplateCategory::Finance,
    default_port: 8282,
    compose: r#"
services:
  wallos:
    image: bellamy9/wallos:latest
    restart: unless-stopped
    expose:
      - "8282"
    volumes:
      - db:/var/www/html/db

volumes:
  db:
"#,
    variables: &[],
};
