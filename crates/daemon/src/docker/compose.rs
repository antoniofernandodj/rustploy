//! Serviços e jobs Docker Compose: `up` de stack com a rede do projeto
//! injetada, execução one-shot de job com cancelamento, e login no registry
//! embutido.

use crate::{db::Db, event_bus::EventBus};
use anyhow::{Result, anyhow};
use bollard::{Docker, volume::CreateVolumeOptions};
use chrono::Utc;
use shared::{Event, RustployConfig};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::{File, remove_file};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tracing::info;

const PROJECT_NET_ALIAS: &str = "rp_project_net";

/// Loga no registry embutido com o token interno `rp-internal` antes de um
/// `docker compose up` (que pode dar pull de imagens de lá). Diferente do
/// pull via bollard em `docker/images.rs` (credenciais passadas por chamada),
/// o `docker compose` é um subprocesso CLI que só entende credenciais via
/// `~/.docker/config.json` — por isso precisa de login/logout explícitos.
/// Best-effort: falha aqui não impede a tentativa de `up` (só vai falhar o
/// pull depois, com erro visível nos logs, se a imagem realmente for da lá).
async fn registry_login(token: &str) -> Result<()> {
    let port = RustployConfig::global().registry.port;
    let mut child = Command::new("docker")
        .args([
            "login",
            &format!("127.0.0.1:{port}"),
            "-u",
            "rp-internal",
            "--password-stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker login: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(token.as_bytes()).await.ok();
        stdin.shutdown().await.ok();
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| anyhow!("falha ao aguardar docker login: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker login falhou: {stderr}"));
    }
    Ok(())
}

/// Desloga do registry embutido (limpeza best-effort, chamada sempre que
/// `registry_login` foi tentado, mesmo se o `up` falhou).
async fn registry_logout() -> Result<()> {
    let port = RustployConfig::global().registry.port;
    let status = Command::new("docker")
        .args(["logout", &format!("127.0.0.1:{port}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| anyhow!("falha ao iniciar docker logout: {e}"))?;
    if !status.success() {
        return Err(anyhow!("docker logout falhou"));
    }
    Ok(())
}

/// Reescreve o YAML do Compose para que todo serviço entre também na rede do
/// projeto (declarada como `external`), sem perder as redes que o usuário já
/// declarou.
pub fn inject_project_network(content: &str, network_name: &str) -> Result<String> {
    use serde_yaml::Value;
    let mut doc: Value =
        serde_yaml::from_str(content).map_err(|e| anyhow!("compose YAML inválido: {e}"))?;

    if doc.is_null() {
        doc = Value::Mapping(serde_yaml::Mapping::new());
    }
    let root = doc
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("compose YAML não é um mapping no nível raiz"))?;

    let alias_key = Value::String(PROJECT_NET_ALIAS.to_string());
    let networks_key = Value::String("networks".to_string());
    if !root.contains_key(&networks_key) {
        root.insert(
            networks_key.clone(),
            Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
    let networks = root
        .get_mut(&networks_key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| anyhow!("`networks:` no compose não é um mapping"))?;

    let already_present = networks.contains_key(&alias_key)
        || networks.values().any(|v| {
            v.as_mapping()
                .and_then(|m| m.get(Value::String("name".to_string())))
                .and_then(Value::as_str)
                == Some(network_name)
        });

    if !already_present {
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(Value::String("external".to_string()), Value::Bool(true));
        entry.insert(
            Value::String("name".to_string()),
            Value::String(network_name.to_string()),
        );
        networks.insert(alias_key.clone(), Value::Mapping(entry));
    }
    let services_key = Value::String("services".to_string());
    if let Some(services_val) = root.get_mut(&services_key) {
        let services = services_val
            .as_mapping_mut()
            .ok_or_else(|| anyhow!("`services:` no compose não é um mapping"))?;

        let svc_net_key = Value::String("networks".to_string());
        for (_, svc) in services.iter_mut() {
            let Some(svc_map) = svc.as_mapping_mut() else {
                continue;
            };
            match svc_map.get_mut(&svc_net_key) {
                Some(Value::Sequence(seq)) => {
                    if !seq.iter().any(|v| v.as_str() == Some(PROJECT_NET_ALIAS)) {
                        seq.push(Value::String(PROJECT_NET_ALIAS.to_string()));
                    }
                }
                Some(Value::Mapping(map)) => {
                    if !map.contains_key(&alias_key) {
                        map.insert(
                            alias_key.clone(),
                            Value::Mapping(serde_yaml::Mapping::new()),
                        );
                    }
                }
                Some(other) if other.is_null() => {
                    *other = Value::Sequence(vec![Value::String(PROJECT_NET_ALIAS.to_string())]);
                }
                Some(_) => {}
                None => {
                    svc_map.insert(
                        svc_net_key.clone(),
                        Value::Sequence(vec![Value::String(PROJECT_NET_ALIAS.to_string())]),
                    );
                }
            }
        }
    }
    serde_yaml::to_string(&doc).map_err(|e| anyhow!("falha ao serializar compose YAML: {e}"))
}

/// Garante que todo volume declarado `external: true` no compose já exista no
/// Docker antes do `up` — o Compose se recusa a criar volumes externos, então
/// se o volume tiver sido removido por fora (ex.: prune manual) o deploy
/// falharia sempre com "external volume ... not found". Idempotente: só cria
/// o que estiver faltando, nunca mexe em volume já existente. Mesmo idioma de
/// `networks::ensure_project_network` (ensure-then-create), aplicado a volumes.
pub async fn ensure_external_volumes(docker: &Docker, content: &str) -> Result<()> {
    use serde_yaml::Value;

    let doc: Value =
        serde_yaml::from_str(content).map_err(|e| anyhow!("compose YAML inválido: {e}"))?;

    let Some(volumes) = doc.get("volumes").and_then(Value::as_mapping) else {
        return Ok(());
    };

    for (key, def) in volumes {
        let Some(map) = def.as_mapping() else {
            continue;
        };

        let is_external = map
            .get(Value::String("external".to_string()))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !is_external {
            continue;
        }

        let name = map
            .get(Value::String("name".to_string()))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| key.as_str().map(str::to_string));
        let Some(name) = name else {
            continue;
        };

        if docker.inspect_volume(&name).await.is_ok() {
            info!(volume = %name, "compose::ensure_external_volumes: volume já existe");
            continue;
        }

        info!(volume = %name, "compose::ensure_external_volumes: criando volume externo ausente");
        docker
            .create_volume(CreateVolumeOptions {
                name: name.clone(),
                ..Default::default()
            })
            .await?;
    }

    Ok(())
}

/// Sobe (ou atualiza) a stack Compose de um serviço: grava `.env` e
/// `docker-compose.yml` com a rede do projeto injetada e roda `docker compose
/// up -d`, com o progresso no log de build.
pub async fn up(
    docker: &Docker,
    content: &str,
    project_name: &str,
    service_id: &str,
    deployment_id: &str,
    network_name: &str,
    bus: &Arc<EventBus>,
    db: &Arc<Db>,
    env_vars: &[(String, String)],
    build_dir: &Path,
    registry_internal_token: Option<Arc<str>>,
) -> Result<()> {
    info!(project = %project_name, "compose_up: iniciando docker compose up");

    let content = inject_project_network(content, network_name)?;
    ensure_external_volumes(docker, &content).await?;

    // Garantir diretório
    tokio::fs::create_dir_all(build_dir).await?;

    // Criar arquivos .env e docker-compose.yml
    let env_file_path = build_dir.join(".env");
    let compose_file_path = build_dir.join("docker-compose.yml");

    {
        let mut env_file = File::create(&env_file_path).await?;
        for (k, v) in env_vars {
            env_file
                .write_all(format!("{}={}\n", k, v).as_bytes())
                .await?;
        }
        env_file.flush().await?;
    } // Aqui o arquivo é fechado

    let mut compose_file = File::create(&compose_file_path).await?;
    compose_file.write_all(content.as_bytes()).await?;
    compose_file.flush().await?;
    drop(compose_file);

    if let Some(token) = &registry_internal_token {
        if let Err(e) = registry_login(token).await {
            tracing::warn!(error = %e, project = %project_name, "compose_up: falha ao autenticar no registry embutido, pull vai falhar se a imagem for de lá");
        }
    }

    let mut child = Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            "docker-compose.yml",
            "--env-file",
            ".env",
            "up",
            "-d",
            "--build",
            "--remove-orphans",
        ])
        .current_dir(build_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose: {e}"))?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    let bus_s = bus.clone();
    let db_s = db.clone();
    let sid = service_id.to_string();
    let did = deployment_id.to_string();
    let read_stdout = async move {
        let mut lines = stdout.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let ts = Utc::now();
            bus_s.publish(Event::BuildLog {
                deployment_id: did.clone(),
                service_id: sid.clone(),
                line: line.clone(),
                timestamp: ts,
            });
            let _ = crate::db::build_logs::append(&db_s, &did, &line, ts).await;
        }
    };
    let bus_e = bus.clone();
    let db_e = db.clone();
    let sid_e = service_id.to_string();
    let did_e = deployment_id.to_string();
    let read_stderr = async move {
        let mut lines = stderr.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let ts = Utc::now();
            bus_e.publish(Event::BuildLog {
                deployment_id: did_e.clone(),
                service_id: sid_e.clone(),
                line: line.clone(),
                timestamp: ts,
            });
            let _ = crate::db::build_logs::append(&db_e, &did_e, &line, ts).await;
        }
    };

    tokio::join!(read_stdout, read_stderr);
    let status = child
        .wait()
        .await
        .map_err(|e| anyhow!("falha ao aguardar: {e}"))?;

    if registry_internal_token.is_some() {
        if let Err(e) = registry_logout().await {
            tracing::warn!(error = %e, project = %project_name, "compose_up: falha ao deslogar do registry embutido (best-effort)");
        }
    }

    // Limpeza
    let _ = remove_file(&env_file_path).await;
    let _ = remove_file(&compose_file_path).await;

    if !status.success() {
        return Err(anyhow!("docker compose up falhou"));
    }
    Ok(())
}

/// Sobe um stack docker-compose UMA VEZ (job one-shot, não um serviço de vida
/// longa): usa `--abort-on-container-exit --exit-code-from <main_service>` —
/// a própria flag do `docker compose` feita pra isso — em vez de `-d`, então
/// o processo espera `main_service` terminar e propaga o exit code dele.
/// Sempre roda `down(...)` depois (sucesso ou falha), pra não deixar
/// containers/redes zumbis do job. Retorna o exit code de `main_service`.
///
/// `mirror_deployment`, quando `Some((deployment_id, service_id))`, também
/// espelha cada linha como `Event::BuildLog`/`build_log` do deployment —
/// usado pelo `DeployExecutor` no estado `PreDeployCheck` pra a saída do
/// job aparecer na tela de deploy, além do histórico normal do Job.
/// De onde vem o `docker-compose.yml` de um `run_once`. `Compose` é o modo
/// original (cola o YAML na mão); `Git` é usado quando `Job.git_source` é
/// `Some` — o repo já foi clonado pra dentro de `build_dir` (ver
/// `jobs::runner`) antes de chegar aqui, então só falta ler o arquivo.
pub enum JobBuildSource<'a> {
    Compose(&'a str),
    /// Caminho relativo a `build_dir` de um `docker-compose.yml` já presente
    /// no clone (ex.: `"docker-compose.yml"` ou `"ci/docker-compose.yml"`).
    Git {
        compose_rel_path: &'a str,
    },
}

/// Exit code sentinel pra "job_run cancelado pelo usuário" (`JobRunCancel`)
/// — distinto do `-1` genérico (processo morto por sinal por outro motivo),
/// pra a UI poder rotular "Cancelado" em vez de "Falhou". Não exige mudança
/// de schema: `job_run.exit_code` já é um inteiro livre. Ver
/// `docs/plano-cancelamento-de-jobs.md`.
pub const CANCELLED_EXIT_CODE: i32 = -2;

/// Espera até o sinal de cancelamento virar `true` (ou o sender ser
/// derrubado, o que só acontece se a task inteira já tiver sumido — nesse
/// caso não há mais nada a cancelar). `watch::Receiver` rastreia versão, não
/// só o valor: mesmo que `true` já tenha sido enviado ANTES desta chamada
/// começar a observar, `changed()` retorna na hora — sem race de "perder" um
/// cancelamento que chegou cedo demais.
async fn wait_for_cancel(rx: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *rx.borrow() {
            return;
        }
        if rx.changed().await.is_err() {
            return;
        }
    }
}

/// Executa um job uma vez: prepara o compose (colado ou do repositório git) com
/// a rede do projeto, roda `run_once_up` e sempre desmonta a stack no fim,
/// mesmo se falhou.
pub async fn run_once(
    source: JobBuildSource<'_>,
    project_name: &str,
    network_name: &str,
    main_service: &str,
    job_id: &str,
    job_run_id: &str,
    bus: &Arc<EventBus>,
    db: &Arc<Db>,
    env_vars: &[(String, String)],
    build_dir: &Path,
    registry_internal_token: Option<Arc<str>>,
    mirror_deployment: Option<(String, String)>,
    cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<i32> {
    info!(project = %project_name, job_id = %job_id, "compose_run_once: iniciando job one-shot");

    tokio::fs::create_dir_all(build_dir).await?;

    // Path relativo (ao `build_dir`) do compose de verdade — fixo pro modo
    // colado; configurável no modo Git (repo pode ter o compose num
    // subdiretório). O conteúdo lido/colado é reescrito no MESMO lugar após
    // `inject_project_network` — nunca movido, pra `build:` relativo no
    // compose do usuário continuar resolvendo certo (o Compose resolve
    // contexto de build relativo à posição do arquivo `-f`, não do cwd).
    let (raw_content, compose_rel_path): (String, String) = match source {
        JobBuildSource::Compose(content) => (content.to_string(), "docker-compose.yml".to_string()),
        JobBuildSource::Git { compose_rel_path } => {
            let path = build_dir.join(compose_rel_path);
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| anyhow!("compose não encontrado em {}: {e}", path.display()))?;
            (content, compose_rel_path.to_string())
        }
    };
    let content = inject_project_network(&raw_content, network_name)?;

    let env_file_path = build_dir.join(".env");
    let compose_file_path = build_dir.join(&compose_rel_path);

    {
        let mut env_file = File::create(&env_file_path).await?;
        for (k, v) in env_vars {
            env_file
                .write_all(format!("{}={}\n", k, v).as_bytes())
                .await?;
        }
        env_file.flush().await?;
    }

    let mut compose_file = File::create(&compose_file_path).await?;
    compose_file.write_all(content.as_bytes()).await?;
    compose_file.flush().await?;
    drop(compose_file);

    if let Some(token) = &registry_internal_token {
        if let Err(e) = registry_login(token).await {
            tracing::warn!(error = %e, project = %project_name, job_id = %job_id, "compose_run_once: falha ao autenticar no registry embutido, pull vai falhar se a imagem for de lá");
        }
    }

    let up_result = run_once_up(
        project_name,
        &compose_rel_path,
        main_service,
        job_id,
        job_run_id,
        bus,
        db,
        build_dir,
        mirror_deployment,
        cancel_rx,
    )
    .await;

    if registry_internal_token.is_some() {
        if let Err(e) = registry_logout().await {
            tracing::warn!(error = %e, project = %project_name, job_id = %job_id, "compose_run_once: falha ao deslogar do registry embutido (best-effort)");
        }
    }

    // Sempre desmonta o stack, mesmo se o up falhou — nunca deixa
    // containers/redes zumbis do job pra trás.
    if let Err(e) = down(&content, project_name, network_name, env_vars).await {
        tracing::warn!(project = %project_name, job_id = %job_id, error = %e, "compose_run_once: down falhou (limpeza best-effort)");
    }
    let _ = remove_file(&env_file_path).await;
    let _ = remove_file(&compose_file_path).await;

    up_result
}

/// Quantas linhas do próprio `docker compose` (build, pull, erros de
/// container) o job guarda pra despejar no log quando falha.
const COMPOSE_TAIL_MAX: usize = 200;

/// Separa o prefixo `<service>-<replica> |` que o `docker compose up` (sem
/// `-d`) põe em cada linha de saída de container. Devolve o nome do serviço
/// e o resto da linha; `None` pra linha sem esse prefixo — as linhas de
/// controle do próprio compose (`Container x Started`, build, spinners).
fn split_service_prefix(line: &str) -> Option<(&str, &str)> {
    let (prefix, rest) = line.split_once('|')?;
    let prefix = prefix.trim();
    let (name, replica) = prefix.rsplit_once('-')?;
    if replica.is_empty() || !replica.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((name, rest.trim_start_matches(' ')))
}

/// Filtra uma linha bruta de `docker compose up` (sem `-d`) pra só sobrar a
/// saída do `main_service`: sem `-d`, o compose intercala stdout/stderr de
/// TODOS os serviços do stack (prefixo `<service>-<replica> | <msg>`, ex.:
/// `main-1  | hello`) e ainda emite linhas de controle próprias (`Container
/// x Started`, spinners com códigos ANSI) sem esse prefixo — tudo isso
/// deixa "Ver logs" do job ilegível quando o compose tem mais de um serviço
/// (ex.: um serviço gatilho + um banco auxiliar). Só a linha de conteúdo do
/// `main_service` sobrevive (prefixo removido); linhas de outros serviços e
/// linhas de controle do compose (sem esse prefixo) são descartadas.
fn filter_main_service_line<'a>(line: &'a str, main_service: &str) -> Option<&'a str> {
    match split_service_prefix(line)? {
        (name, rest) if name == main_service => Some(rest),
        _ => None,
    }
}

/// Limpa uma linha de controle do compose pra caber no log do job: fica só
/// o último quadro de um spinner (texto depois do último `\r`) e sem os
/// códigos ANSI de cor/cursor. `None` quando não sobra texto.
fn clean_compose_line(line: &str) -> Option<String> {
    let line = line.rsplit('\r').next().unwrap_or(line);
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        // CSI (`ESC [ ... <final>`): pula até o byte final (0x40..=0x7E).
        // Outros escapes de dois caracteres (`ESC x`): pula só o `x`.
        if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    break;
                }
            }
        }
    }
    let out = out.trim_end();
    (!out.trim().is_empty()).then(|| out.to_string())
}

/// Grava uma linha no log do job (tabela `job_log` + evento ao vivo) e, se o
/// job espelha um deployment, no build log dele também.
async fn record_job_line(
    bus: &EventBus,
    db: &Db,
    job_id: &str,
    job_run_id: &str,
    mirror: Option<&(String, String)>,
    stream: shared::protocol::LogStream,
    line: String,
) {
    let ts = Utc::now();
    bus.publish(Event::JobLogLine {
        job_run_id: job_run_id.to_string(),
        job_id: job_id.to_string(),
        line: line.clone(),
        timestamp: ts,
        stream: stream.clone(),
    });
    let _ = crate::db::job_log::append(db, job_run_id, &stream, &line, ts).await;
    if let Some((deployment_id, service_id)) = mirror {
        let _ = crate::db::build_logs::append(db, deployment_id, &line, ts).await;
        bus.publish(Event::BuildLog {
            deployment_id: deployment_id.clone(),
            service_id: service_id.clone(),
            line,
            timestamp: ts,
        });
    }
}

/// Lê um dos pipes do `docker compose up` até fechar. A saída do
/// `main_service` vai direto pro log do job; a de outros serviços é
/// descartada; as linhas do próprio compose vão pro `compose_tail`, que só
/// é despejado no log se o job falhar — é onde mora a causa quando o stack
/// nem chega a rodar o `main_service` (build quebrado, porta ocupada…).
#[allow(clippy::too_many_arguments)]
async fn read_compose_output<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    stream: shared::protocol::LogStream,
    main_service: &str,
    bus: &EventBus,
    db: &Db,
    job_id: &str,
    job_run_id: &str,
    mirror: Option<&(String, String)>,
    compose_tail: &Mutex<VecDeque<String>>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(raw_line)) = lines.next_line().await {
        if let Some(line) = filter_main_service_line(&raw_line, main_service) {
            if !line.trim().is_empty() {
                let line = line.to_string();
                record_job_line(bus, db, job_id, job_run_id, mirror, stream.clone(), line).await;
            }
            continue;
        }
        if split_service_prefix(&raw_line).is_some() {
            continue;
        }
        if let Some(line) = clean_compose_line(&raw_line) {
            let mut tail = compose_tail.lock().unwrap_or_else(|e| e.into_inner());
            if tail.len() == COMPOSE_TAIL_MAX {
                tail.pop_front();
            }
            tail.push_back(line);
        }
    }
}

/// Roda o `docker compose up` de um job até o `main_service` sair, transmitindo
/// stdout/stderr para o log e matando o processo se chegar o sinal de
/// cancelamento.
async fn run_once_up(
    project_name: &str,
    compose_rel_path: &str,
    main_service: &str,
    job_id: &str,
    job_run_id: &str,
    bus: &Arc<EventBus>,
    db: &Arc<Db>,
    build_dir: &Path,
    mirror_deployment: Option<(String, String)>,
    cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<i32> {
    let mut child = Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            compose_rel_path,
            "--env-file",
            ".env",
            "up",
            "--build",
            "--abort-on-container-exit",
            "--exit-code-from",
            main_service,
            "--remove-orphans",
        ])
        .current_dir(build_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose: {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mirror = mirror_deployment.as_ref();
    let compose_tail = Mutex::new(VecDeque::new());
    let read_stdout = read_compose_output(
        stdout,
        shared::protocol::LogStream::Stdout,
        main_service,
        bus,
        db,
        job_id,
        job_run_id,
        mirror,
        &compose_tail,
    );
    let read_stderr = read_compose_output(
        stderr,
        shared::protocol::LogStream::Stderr,
        main_service,
        bus,
        db,
        job_id,
        job_run_id,
        mirror,
        &compose_tail,
    );

    // Sem cancelamento: espera os readers normalmente (eles só terminam
    // quando os pipes fecham, ou seja, quando o processo sai). Com
    // cancelamento: corre os readers contra o sinal — se o sinal vencer,
    // mata o processo (fecha os pipes, os readers terminam sozinhos) em vez
    // de esperar `read_stdout`/`read_stderr` até o fim (que só terminariam
    // quando o processo sair por conta própria — exatamente o que o
    // cancelamento quer evitar).
    let mut cancelado = false;
    match cancel_rx {
        Some(mut rx) => {
            tokio::select! {
                _ = async { tokio::join!(read_stdout, read_stderr) } => {}
                _ = wait_for_cancel(&mut rx) => {
                    cancelado = true;
                    let _ = child.start_kill();
                }
            }
        }
        None => {
            tokio::join!(read_stdout, read_stderr);
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| anyhow!("falha ao aguardar: {e}"))?;

    if cancelado {
        return Ok(CANCELLED_EXIT_CODE);
    }

    // `--exit-code-from` propaga o exit code de `main_service` como o do
    // processo `docker compose` — sem código (ex.: morto por sinal) conta
    // como falha (-1), não sucesso silencioso.
    let exit_code = status.code().unwrap_or(-1);

    // Na falha, o fim da saída do próprio compose entra no log: sem isso,
    // um stack que quebra antes do `main_service` rodar (build, porta
    // ocupada, container com nome repetido) termina com exit code != 0 e
    // log vazio. No sucesso fica de fora, pra não poluir "Ver logs".
    if exit_code != 0 {
        let tail = std::mem::take(&mut *compose_tail.lock().unwrap_or_else(|e| e.into_inner()));
        if !tail.is_empty() {
            let header = format!(
                "==> Job falhou (exit code {exit_code}); últimas {} linhas do docker compose:",
                tail.len()
            );
            record_job_line(
                bus,
                db,
                job_id,
                job_run_id,
                mirror,
                shared::protocol::LogStream::Stderr,
                header,
            )
            .await;
            for line in tail {
                record_job_line(
                    bus,
                    db,
                    job_id,
                    job_run_id,
                    mirror,
                    shared::protocol::LogStream::Stderr,
                    line,
                )
                .await;
            }
        }
    }

    Ok(exit_code)
}

pub async fn down(
    content: &str,
    project_name: &str,
    _network_name: &str,
    env_vars: &[(String, String)],
) -> Result<()> {
    info!(project = %project_name, "compose_down: iniciando");
    let mut child = Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            "-",
            "down",
            "--remove-orphans",
        ])
        .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose down: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes()).await;
        stdin.shutdown().await.ok();
    }
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| anyhow!("falha ao aguardar compose down: {e}"))?;

    if !output.status.success() {
        return Err(anyhow!("docker compose down falhou"));
    }
    info!(project = %project_name, "compose_down: concluído");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_and_strips_main_service_line() {
        assert_eq!(
            filter_main_service_line("main-1  | hello world", "main"),
            Some("hello world")
        );
    }

    #[test]
    fn drops_other_service_lines() {
        assert_eq!(
            filter_main_service_line("side-1  | hello world", "main"),
            None
        );
    }

    #[test]
    fn keeps_main_service_with_hyphen_in_name() {
        assert_eq!(
            filter_main_service_line("my-service-1  | booted", "my-service"),
            Some("booted")
        );
    }

    #[test]
    fn drops_compose_control_lines_without_pipe() {
        assert_eq!(
            filter_main_service_line(" Container rp-main-1 Started ", "main"),
            None
        );
        assert_eq!(
            filter_main_service_line("main-1 exited with code 0", "main"),
            None
        );
    }

    #[test]
    fn drops_lines_without_replica_suffix() {
        assert_eq!(filter_main_service_line("main | hello", "main"), None);
    }

    #[test]
    fn detects_prefix_of_any_service() {
        assert_eq!(
            split_service_prefix("side-1  | hello"),
            Some(("side", "hello"))
        );
        assert_eq!(split_service_prefix(" Container rp-main-1 Started "), None);
    }

    #[test]
    fn cleans_ansi_and_spinner_frames_from_compose_lines() {
        assert_eq!(
            clean_compose_line(
                "\u{1b}[1A\u{1b}[2K\u{1b}[31mError\u{1b}[0m: port is already allocated"
            ),
            Some("Error: port is already allocated".to_string())
        );
        assert_eq!(
            clean_compose_line(" ⠋ Container a  Starting\r ✔ Container a  Started  "),
            Some(" ✔ Container a  Started".to_string())
        );
        assert_eq!(clean_compose_line("\u{1b}[2K   "), None);
    }

    #[tokio::test]
    async fn keeps_only_compose_lines_in_tail() {
        let bus = EventBus::new();
        let db = crate::db::Db::connect_lazy("sqlite::memory:").unwrap();
        let tail = Mutex::new(VecDeque::new());
        let input: &[u8] =
            b"main-1  | hello\nside-1  | ignored\n\x1b[31mfailed to bind port 5432\x1b[0m\n\n";
        read_compose_output(
            input,
            shared::protocol::LogStream::Stderr,
            "main",
            &bus,
            &db,
            "job_x",
            "jrun_x",
            None,
            &tail,
        )
        .await;
        assert_eq!(
            tail.into_inner().unwrap(),
            VecDeque::from(["failed to bind port 5432".to_string()])
        );
    }

    #[tokio::test]
    async fn wait_for_cancel_returns_immediately_if_already_true() {
        // Cobre o caso "cancelamento chegou antes de alguém observar":
        // watch::Receiver rastreia versão, então mesmo enviado ANTES desta
        // chamada começar, `changed()` não perde o sinal.
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        tx.send(true).unwrap();
        tokio::time::timeout(
            std::time::Duration::from_millis(200),
            wait_for_cancel(&mut rx),
        )
        .await
        .expect("wait_for_cancel não deveria bloquear quando já está true");
    }

    #[tokio::test]
    async fn wait_for_cancel_resolves_when_sent_later() {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        let waiter = tokio::spawn(async move {
            wait_for_cancel(&mut rx).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tx.send(true).unwrap();
        tokio::time::timeout(std::time::Duration::from_millis(200), waiter)
            .await
            .expect("wait_for_cancel deveria resolver após o send")
            .unwrap();
    }

    #[tokio::test]
    async fn wait_for_cancel_returns_when_sender_dropped() {
        // Sender derrubado sem nunca enviar `true` (task já sumiu) — não
        // pode travar pra sempre.
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        drop(tx);
        tokio::time::timeout(
            std::time::Duration::from_millis(200),
            wait_for_cancel(&mut rx),
        )
        .await
        .expect("wait_for_cancel não deveria travar com o sender derrubado");
    }
}
