use super::AppState;
use postcard;
use axum::{
    body::Body,
    extract::{Query, State},
    http::header,
    response::Response,
};
use serde::Deserialize;
use shared::Event;
use tokio::sync::broadcast::error::RecvError;

#[derive(Deserialize)]
pub struct StreamQuery {
    pub service: Option<String>,
}

pub async fn handler(State(state): State<AppState>, Query(q): Query<StreamQuery>) -> Response {
    let mut rx = state.bus.subscribe();
    let service_filter = q.service;

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(ref sid) = service_filter {
                        if !event_matches(sid, &event) {
                            continue;
                        }
                    }
                    if let Ok(payload) = postcard::to_allocvec(&event) {
                        let len = (payload.len() as u32).to_le_bytes();
                        let mut frame = Vec::with_capacity(4 + payload.len());
                        frame.extend_from_slice(&len);
                        frame.extend_from_slice(&payload);
                        yield Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(frame));
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from_stream(stream))
        .unwrap()
}

fn event_matches(service_id: &str, event: &Event) -> bool {
    match event {
        Event::DeployStateChanged { service_id: sid, .. } => sid == service_id,
        Event::DeployProgress { service_id: sid, .. } => sid == service_id,
        Event::BuildLog { service_id: sid, .. } => sid == service_id,
        Event::LogLine { service_id: sid, .. } => sid == service_id,
        Event::ContainerMetrics(m) => m.service_id == service_id,
        Event::ServiceStatusChanged { service_id: sid, .. } => sid == service_id,
        Event::DaemonReady { .. } | Event::Error { .. } => true,
    }
}
