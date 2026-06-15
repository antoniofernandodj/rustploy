//! Background connection worker, exposed to iced as a `Subscription` stream.
//!
//! Owns two RWP connections — one for request/response RPCs and one for the
//! event stream — and bridges them to the UI through iced messages.

use crate::rwp;
use crate::{Ctx, Message, WorkerEvent};
use iced::futures::{SinkExt, Stream};
use shared::{Command, RwpFrame};
use tokio::sync::mpsc;

pub fn connect(addr: String, token: Option<String>) -> impl Stream<Item = Message> {
    iced::stream::channel(64, move |mut output| async move {
        macro_rules! emit {
            ($ev:expr) => {
                let _ = output.send(Message::Worker($ev)).await;
            };
        }

        let mut cmd_conn = match rwp::connect(&addr, token.as_deref()).await {
            Ok(s) => s,
            Err(e) => {
                emit!(WorkerEvent::Error(e.to_string()));
                return;
            }
        };

        let mut evt_conn = match rwp::connect(&addr, token.as_deref()).await {
            Ok(s) => s,
            Err(e) => {
                emit!(WorkerEvent::Error(e.to_string()));
                return;
            }
        };
        if let Err(e) =
            rwp::write_frame(&mut evt_conn, &RwpFrame::Subscribe { service_id: None }).await
        {
            emit!(WorkerEvent::Error(e.to_string()));
            return;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<(Ctx, Command)>();
        emit!(WorkerEvent::Ready(tx));
        emit!(WorkerEvent::Connected);

        loop {
            tokio::select! {
                maybe = rx.recv() => match maybe {
                    Some((ctx, command)) => match rwp::rpc(&mut cmd_conn, command).await {
                        Ok(resp) => { emit!(WorkerEvent::Reply(ctx, resp)); }
                        Err(e) => { emit!(WorkerEvent::Error(e.to_string())); return; }
                    },
                    None => return,
                },
                frame = rwp::read_frame::<shared::RwpReply>(&mut evt_conn) => match frame {
                    Ok(shared::RwpReply::Event(ev)) => { emit!(WorkerEvent::Event(ev)); }
                    Ok(_) => {}
                    Err(_) => { emit!(WorkerEvent::Disconnected); return; }
                },
            }
        }
    })
}
