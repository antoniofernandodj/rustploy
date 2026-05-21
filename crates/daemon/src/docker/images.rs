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
    info!(image, "pulling image");
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
                    if status.contains("Pull complete") || status.contains("Already exists") {
                        layers_done += 1;
                    }
                    if status.contains("Pulling from") {
                        layer_count += 1;
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
                        description: status.clone(),
                    });
                }
            }
            Err(e) => return Err(anyhow!("image pull failed: {e}")),
        }
    }

    info!(image, "image pull complete");
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
        tar.append_dir_all(".", context_path)?;
        let enc = tar.into_inner()?;
        enc.finish()?;
    }
    Ok(tar_data)
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
