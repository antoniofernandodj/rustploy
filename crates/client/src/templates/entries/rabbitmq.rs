use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "rabbitmq",
    name: "RabbitMQ",
    description: "Broker de mensageria multi-protocolo para comunicação assíncrona",
    category: TemplateCategory::Database,
    default_port: 5672,
    compose: r#"
services:
  rabbitmq:
    image: rabbitmq:management
    restart: unless-stopped
    ports:
      - "5672"
    environment:
      RABBITMQ_DEFAULT_USER: {{RABBITMQ_DEFAULT_USER}}
      RABBITMQ_DEFAULT_PASS: {{RABBITMQ_DEFAULT_PASS}}
    volumes:
      - data:/var/lib/rabbitmq

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "RABBITMQ_DEFAULT_USER",
            label: "Usuário",
            default: Some("guest"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "RABBITMQ_DEFAULT_PASS",
            label: "Senha",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
