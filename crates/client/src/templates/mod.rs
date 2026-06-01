pub mod catalog;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemplateCategory {
    All,
    Cms,
    Analytics,
    Monitoring,
    DevTools,
    Communication,
    Storage,
    Security,
    Automation,
    Media,
}

impl TemplateCategory {
    pub const FILTERS: &'static [TemplateCategory] = &[
        Self::All,
        Self::Cms,
        Self::Analytics,
        Self::Monitoring,
        Self::DevTools,
        Self::Communication,
        Self::Storage,
        Self::Security,
        Self::Automation,
        Self::Media,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All          => "Todos",
            Self::Cms          => "CMS",
            Self::Analytics    => "Analytics",
            Self::Monitoring   => "Monitoring",
            Self::DevTools     => "DevTools",
            Self::Communication => "Chat",
            Self::Storage      => "Storage",
            Self::Security     => "Segurança",
            Self::Automation   => "Automação",
            Self::Media        => "Mídia",
        }
    }
}

pub struct TemplateVar {
    pub key: &'static str,
    pub label: &'static str,
    pub default: Option<&'static str>,
    pub required: bool,
    pub secret: bool,
}

impl std::fmt::Debug for TemplateVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TemplateVar({})", self.key)
    }
}

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: TemplateCategory,
    pub default_port: u16,
    pub compose: &'static str,
    pub variables: &'static [TemplateVar],
}

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Template({})", self.id)
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub fn all() -> &'static [Template] {
    catalog::TEMPLATES
}

pub fn filtered(category: TemplateCategory, search: &str) -> Vec<&'static Template> {
    let search = search.to_lowercase();
    catalog::TEMPLATES
        .iter()
        .filter(|t| {
            let cat_ok = category == TemplateCategory::All || t.category == category;
            let search_ok = search.is_empty()
                || t.name.to_lowercase().contains(&search)
                || t.description.to_lowercase().contains(&search);
            cat_ok && search_ok
        })
        .collect()
}

/// Substitutes {{KEY}} placeholders in the compose template with user-provided values.
/// Falls back to the variable's default when the value is empty.
pub fn render_compose(template: &'static Template, values: &[String]) -> String {
    let mut out = template.compose.to_string();
    for (var, val) in template.variables.iter().zip(values.iter()) {
        let placeholder = format!("{{{{{}}}}}", var.key);
        let replacement = if val.is_empty() {
            var.default.unwrap_or("").to_string()
        } else {
            val.clone()
        };
        out = out.replace(&placeholder, &replacement);
    }
    out
}
