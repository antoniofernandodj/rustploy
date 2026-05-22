use crate::event_bus::EventBus;
use anyhow::{anyhow, Result};
use bollard::{
    image::{BuildImageOptions, CreateImageOptions, RemoveImageOptions},
    Docker,
};
use futures::StreamExt;
use shared::Event;
use std::path::Path;
use tracing::{debug, info};

pub async fn pull(
    docker: &Docker,
    image: &str,
    service_id: &str,
    deployment_id: &str,
    bus: &EventBus,
) -> Result<()> {
    info!(image = %image, deployment_id = %deployment_id, "images::pull: iniciando pull");
    let options = Some(CreateImageOptions {
        from_image: image,
        ..Default::default()
    });

    let mut stream = docker.create_image(options, None, None);
    let mut layer_count = 0u32;
    let mut layers_done = 0u32;

    while let Some(item) = stream.next().await {
        match item {
            Ok(info) => {
                if let Some(status) = &info.status {
                    let layer_id = info.id.as_deref().unwrap_or("-");
                    if status.contains("Pull complete") || status.contains("Already exists") {
                        layers_done += 1;
                        debug!(
                            image = %image,
                            layer = %layer_id,
                            status = %status,
                            done = layers_done,
                            "images::pull: layer concluída"
                        );
                    }
                    if status.contains("Pulling from") {
                        layer_count += 1;
                        info!(image = %image, "images::pull: baixando layers do registry");
                    }
                    let percent = if layer_count > 0 {
                        ((layers_done as f32 / layer_count as f32) * 100.0) as u8
                    } else {
                        0
                    };
                    bus.publish(Event::DeployProgress {
                        deployment_id: deployment_id.to_string(),
                        service_id: service_id.to_string(),
                        phase: "PullingImage".into(),
                        percent,
                        description: format!("[{layer_id}] {status}"),
                    });
                }
            }
            Err(e) => {
                tracing::error!(image = %image, error = %e, "images::pull: falhou");
                return Err(anyhow!("image pull failed: {e}"));
            }
        }
    }

    info!(image = %image, deployment_id = %deployment_id, layers_done = layers_done, "images::pull: pull concluído");
    Ok(())
}

pub async fn exists(docker: &Docker, image: &str) -> bool {
    docker.inspect_image(image).await.is_ok()
}

pub async fn build(
    docker: &Docker,
    context_path: &Path,
    dockerfile: &str,
    tag: &str,
    service_id: &str,
    deployment_id: &str,
    bus: &EventBus,
) -> Result<()> {
    use chrono::Utc;

    info!(tag, "building image from context");

    let tar_gz = create_tar_gz(context_path, dockerfile)?;
    let options = BuildImageOptions {
        dockerfile: dockerfile.to_string(),
        t: tag.to_string(),
        rm: true,
        forcerm: true,
        ..Default::default()
    };

    let mut stream = docker.build_image(options, None, Some(tar_gz.into()));
    while let Some(item) = stream.next().await {
        match item {
            Ok(output) => {
                if let Some(stream) = output.stream {
                    let line = stream.trim_end_matches('\n');
                    if !line.is_empty() {
                        bus.publish(Event::LogLine {
                            service_id: service_id.to_string(),
                            container_id: format!("build:{deployment_id}"),
                            stream: shared::protocol::LogStream::Stdout,
                            line: line.to_string(),
                            timestamp: Utc::now(),
                        });
                    }
                }
                if let Some(err) = output.error {
                    return Err(anyhow!("docker build error: {err}"));
                }
            }
            Err(e) => return Err(anyhow!("docker build stream error: {e}")),
        }
    }

    info!(tag, "image build complete");
    Ok(())
}

fn create_tar_gz(context_path: &Path, _dockerfile: &str) -> Result<Vec<u8>> {
    let mut tar_data = Vec::new();
    {
        let enc = flate2::write::GzEncoder::new(&mut tar_data, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        // Adiciona recursivamente excluindo .git (que pode ser muito grande
        // e não é necessário para o build da imagem Docker)
        append_dir_filtered(&mut tar, context_path, std::path::Path::new("."))?;
        let enc = tar.into_inner()?;
        enc.finish()?;
    }
    Ok(tar_data)
}

/// Adiciona `src` ao tar sob `prefix`, pulando entradas cujo nome seja `.git`.
fn append_dir_filtered(
    tar: &mut tar::Builder<impl std::io::Write>,
    src: &Path,
    prefix: &Path,
) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        // Ignora .git e qualquer ponto de controle de versão indesejado
        if file_name == ".git" {
            continue;
        }
        let path = entry.path();
        let archive_path = prefix.join(&file_name);
        if path.is_dir() {
            append_dir_filtered(tar, &path, &archive_path)?;
        } else if path.is_file() {
            tar.append_path_with_name(&path, &archive_path)
                .map_err(|e| anyhow::anyhow!("tar: falha ao adicionar '{}': {e}", path.display()))?;
        }
        // symlinks são ignorados (seguro para contexto Docker)
    }
    Ok(())
}

pub async fn prune_unused(docker: &Docker, keep_tags: &[&str]) -> Result<()> {
    let filters = std::collections::HashMap::from([("dangling", vec!["false"])]);
    let opts = bollard::image::ListImagesOptions {
        filters: filters
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
            .collect(),
        ..Default::default()
    };
    let images = docker.list_images(Some(opts)).await?;
    for image in images {
        let keep = image.repo_tags.iter().any(|t| keep_tags.contains(&t.as_str()));
        if !keep {
            for tag in &image.repo_tags {
                if tag.starts_with("rp_") {
                    debug!(tag, "pruning unused image");
                    let _ = docker
                        .remove_image(
                            tag,
                            Some(RemoveImageOptions { force: false, noprune: false }),
                            None,
                        )
                        .await;
                }
            }
        }
    }
    Ok(())
}
