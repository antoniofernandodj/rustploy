use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "influxdb",
    name: "InfluxDB",
    description: "Banco de dados otimizado para séries temporais",
    category: TemplateCategory::Database,
    default_port: 8086,
    compose: r#"
services:
  influxdb:
    image: influxdb:latest
    restart: unless-stopped
    expose:
      - "8086"
    environment:
      DOCKER_INFLUXDB_INIT_USERNAME: {{DOCKER_INFLUXDB_INIT_USERNAME}}
      DOCKER_INFLUXDB_INIT_PASSWORD: {{DOCKER_INFLUXDB_INIT_PASSWORD}}
      DOCKER_INFLUXDB_INIT_ORG: {{DOCKER_INFLUXDB_INIT_ORG}}
      DOCKER_INFLUXDB_INIT_BUCKET: {{DOCKER_INFLUXDB_INIT_BUCKET}}
    volumes:
      - data:/var/lib/influxdb2

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "DOCKER_INFLUXDB_INIT_USERNAME",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "DOCKER_INFLUXDB_INIT_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "DOCKER_INFLUXDB_INIT_ORG",
            label: "Organização",
            default: Some("myorg"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "DOCKER_INFLUXDB_INIT_BUCKET",
            label: "Bucket padrão",
            default: Some("mybucket"),
            required: true,
            secret: false,
        },
    ],
};
