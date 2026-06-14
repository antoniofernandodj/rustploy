use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Blocking,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub scope: String,
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub issues: Vec<Issue>,
}

impl Report {
    pub fn blocking(&mut self, scope: impl Into<String>, code: &str, msg: impl Into<String>) {
        self.push(Severity::Blocking, scope, code, msg, None);
    }

    pub fn warn(&mut self, scope: impl Into<String>, code: &str, msg: impl Into<String>, hint: impl Into<String>) {
        self.push(Severity::Warning, scope, code, msg, Some(hint.into()));
    }

    fn push(
        &mut self,
        severity: Severity,
        scope: impl Into<String>,
        code: &str,
        msg: impl Into<String>,
        hint: Option<String>,
    ) {
        self.issues.push(Issue {
            severity,
            scope: scope.into(),
            code: code.to_string(),
            message: msg.into(),
            hint,
        });
    }

    pub fn has_blocking(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Blocking)
    }

    pub fn print(&self) {
        let blocking: Vec<_> = self.issues.iter().filter(|i| i.severity == Severity::Blocking).collect();
        let warnings: Vec<_> = self.issues.iter().filter(|i| i.severity == Severity::Warning).collect();
        let infos: Vec<_> = self.issues.iter().filter(|i| i.severity == Severity::Info).collect();

        if !blocking.is_empty() {
            eprintln!("\n\x1b[31m=== ERROS BLOQUEANTES ({}) ===\x1b[0m", blocking.len());
            for i in &blocking {
                eprintln!("  \x1b[31m[{}]\x1b[0m [{}] {}", i.code, i.scope, i.message);
                if let Some(h) = &i.hint { eprintln!("        → {h}"); }
            }
        }

        if !warnings.is_empty() {
            eprintln!("\n\x1b[33m=== WARNINGS ({}) ===\x1b[0m", warnings.len());
            for i in &warnings {
                eprintln!("  \x1b[33m[{}]\x1b[0m [{}] {}", i.code, i.scope, i.message);
                if let Some(h) = &i.hint { eprintln!("        → {h}"); }
            }
        }

        if !infos.is_empty() {
            eprintln!("\n=== INFO ({}) ===", infos.len());
            for i in &infos {
                eprintln!("  [{}] [{}] {}", i.code, i.scope, i.message);
            }
        }

        if self.issues.is_empty() {
            eprintln!("Nenhum problema encontrado.");
        }
    }
}
