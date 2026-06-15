use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "gitlab-ce",
    name: "GitLab CE",
    description: "Plataforma DevOps completa para código e CI/CD",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  gitlab-ce:
    image: gitlab/gitlab-ce:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      GITLAB_ROOT_PASSWORD: {{GITLAB_ROOT_PASSWORD}}
      GITLAB_OMNIBUS_CONFIG: {{GITLAB_OMNIBUS_CONFIG}}
    volumes:
      - config:/etc/gitlab
      - logs:/var/log/gitlab
      - data:/var/opt/gitlab

volumes:
  config:
  logs:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "GITLAB_ROOT_PASSWORD",
            label: "Senha root",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "GITLAB_OMNIBUS_CONFIG",
            label: "Hostname config",
            default: Some("external_url 'http://localhost'"),
            required: true,
            secret: false,
        },
    ],
};
