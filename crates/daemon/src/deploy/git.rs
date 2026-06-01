use anyhow::{Result, anyhow};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info};

// ── Public types ──────────────────────────────────────────────────────────────

pub struct CloneOptions<'a> {
    pub url: &'a str,
    pub branch: &'a str,
    /// Personal-access token for HTTPS authentication. SSH URLs ignore this.
    pub token: Option<&'a str>,
    pub dir: &'a Path,
}

pub struct CloneProgress {
    pub phase: String,
    pub percent: u8,
    pub description: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Clone a git repository into `opts.dir`, calling `on_progress` for each
/// stderr line emitted by git. Returns `Err` if git exits non-zero.
pub async fn clone(
    opts: CloneOptions<'_>,
    mut on_progress: impl FnMut(CloneProgress),
) -> Result<()> {
    if opts.dir.exists() {
        std::fs::remove_dir_all(opts.dir)
            .map_err(|e| anyhow!("falha ao limpar diretório de clone: {e}"))?;
    }
    std::fs::create_dir_all(opts.dir)?;

    let effective_url = inject_token(opts.url, opts.token);
    info!(
        url = %redact_url(opts.url),
        branch = %opts.branch,
        dir = %opts.dir.display(),
        "git::clone: iniciando"
    );

    let mut child = Command::new("git")
        .args([
            "clone",
            "--branch",
            opts.branch,
            "--progress",
            "--",
            &effective_url,
            ".",
        ])
        .current_dir(opts.dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        // Prevent git from hanging waiting for interactive credential prompts.
        .env("GIT_TERMINAL_PROMPT", "0")
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar git: {e} — git está instalado?"))?;

    let stderr = child.stderr.take().expect("stderr foi capturado acima");
    let mut reader = BufReader::new(stderr);
    let mut line_buf = String::new();
    // Keep the last N stderr lines to surface in the error message on failure.
    let mut tail: Vec<String> = Vec::new();

    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        // Git uses \r (not \n) to overwrite progress lines on a terminal.
        // When piped, both characters appear — split on both.
        for part in line_buf.split(['\r', '\n']) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            debug!(line = %part, "git::clone: stderr");
            on_progress(parse_progress(part));
            if tail.len() >= 10 {
                tail.remove(0);
            }
            tail.push(part.to_string());
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| anyhow!("falha ao aguardar processo git: {e}"))?;

    if !status.success() {
        let detail = tail.join(" | ");
        return Err(anyhow!(
            "git clone falhou (exit {}): {}",
            status.code().unwrap_or(-1),
            detail
        ));
    }

    info!(
        url = %redact_url(opts.url),
        branch = %opts.branch,
        "git::clone: concluído"
    );
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Embeds a personal-access token into an HTTPS URL.
/// SSH and file:// URLs are returned unchanged.
fn inject_token(url: &str, token: Option<&str>) -> String {
    let Some(tok) = token else {
        return url.to_string();
    };
    if let Some(rest) = url.strip_prefix("https://") {
        // Strip existing credentials if the URL already contains them.
        let host_path = rest.split_once('@').map_or(rest, |(_, hp)| hp);
        return format!("https://x-token-auth:{tok}@{host_path}");
    }
    url.to_string()
}

/// Returns the URL with any embedded credentials replaced by `***`.
fn redact_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some((creds, host_path)) = rest.split_once('@') {
            let user = creds.split(':').next().unwrap_or("user");
            return format!("https://{user}:***@{host_path}");
        }
    }
    url.to_string()
}

/// Maps a raw git stderr line to a `CloneProgress` value.
fn parse_progress(line: &str) -> CloneProgress {
    let percent = extract_percent(line);

    let phase = if line.contains("Counting") {
        "Counting objects"
    } else if line.contains("Compressing") {
        "Compressing objects"
    } else if line.contains("Receiving") {
        "Receiving objects"
    } else if line.contains("Resolving") {
        "Resolving deltas"
    } else {
        "Cloning"
    };

    CloneProgress {
        phase: phase.to_string(),
        percent,
        description: line.to_string(),
    }
}

/// Extracts the integer percentage from a git progress line.
/// "Receiving objects:  47% (23/49), ..." → 47
fn extract_percent(line: &str) -> u8 {
    let Some(pct_pos) = line.find('%') else {
        return 0;
    };
    let before = &line[..pct_pos];
    // Walk back from '%' to find the start of the number.
    let num_start = before
        .rfind(|c: char| !c.is_ascii_digit() && c != ' ')
        .map_or(0, |i| i + 1);
    before[num_start..].trim().parse().unwrap_or(0)
}
