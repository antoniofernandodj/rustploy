use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "jenkins",
    name: "Jenkins",
    description: "Servidor de automação open-source para pipelines CI/CD",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  jenkins:
    image: jenkins/jenkins:lts
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - data:/var/jenkins_home

volumes:
  data:
"#,
    variables: &[],
};
