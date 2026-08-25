mod actions;
mod pool;
mod settings;

use std::collections::HashMap;
use std::env;

use actions::{build_job, DeviceJob, Gesture};
use futures::{SinkExt, StreamExt};
use pool::{ConnectionStatus, EndpointKey, Pool, Work};
use settings::{ActionSettings, PiMessage, PiOut};
use streamdeck_rs::registration::RegistrationParams;
use streamdeck_rs::{Message, MessageOut, StreamDeckSocket};
use tokio::sync::{mpsc, oneshot};

type SdSocket = StreamDeckSocket<(), ActionSettings, PiMessage, PiOut>;

enum Outgoing {
    ShowOk {
        context: String,
    },
    ShowAlert {
        context: String,
    },
    ToPi {
        action: String,
        context: String,
        payload: PiOut,
    },
    GetSettings {
        context: String,
    },
    Log {
        message: String,
    },
}

struct Plugin {
    pool: Pool,
    open_pi: HashMap<String, String>,
    outgoing: mpsc::UnboundedSender<Outgoing>,
}

#[tokio::main]
async fn main() {
    let params = RegistrationParams::from_args(env::args()).expect("Stream Deck registration args");
    let mut socket: SdSocket = StreamDeckSocket::connect(params.port, params.event, params.uuid)
        .await
        .expect("connect to Stream Deck");

    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel();
    let (status_tx, mut status_rx) = mpsc::unbounded_channel();
    let (idle_tx, mut idle_rx) = mpsc::unbounded_channel();

    let mut plugin = Plugin {
        pool: Pool::new(status_tx, idle_tx),
        open_pi: HashMap::new(),
        outgoing: outgoing_tx,
    };

    loop {
        tokio::select! {
            msg = socket.next() => {
                match msg {
                    Some(Ok(message)) => plugin.handle(message),
                    Some(Err(e)) => eprintln!("streamdeck read error: {e:?}"),
                    None => break,
                }
            }
            out = outgoing_rx.recv() => {
                let Some(out) = out else { break };
                if let Err(e) = send_out(&mut socket, out).await {
                    eprintln!("streamdeck write error: {e:?}");
                }
            }
            status = status_rx.recv() => {
                let Some((key, status)) = status else { break };
                plugin.on_status(key, status);
            }
            idle = idle_rx.recv() => {
                let Some((key, generation)) = idle else { break };
                plugin.pool.apply_idle(key, generation);
            }
        }
    }
}

async fn send_out(socket: &mut SdSocket, out: Outgoing) -> Result<(), String> {
    let result = match out {
        Outgoing::ShowOk { context } => socket.send(MessageOut::ShowOk { context }).await,
        Outgoing::ShowAlert { context } => socket.send(MessageOut::ShowAlert { context }).await,
        Outgoing::ToPi {
            action,
            context,
            payload,
        } => {
            socket
                .send(MessageOut::SendToPropertyInspector {
                    action,
                    context,
                    payload,
                })
                .await
        }
        Outgoing::GetSettings { context } => socket.send(MessageOut::GetSettings { context }).await,
        Outgoing::Log { message } => {
            socket
                .send(MessageOut::LogMessage {
                    payload: streamdeck_rs::LogMessagePayload { message },
                })
                .await
        }
    };
    result.map_err(|e| format!("{e:?}"))
}

impl Plugin {
    fn handle(&mut self, message: Message<(), ActionSettings, PiMessage>) {
        match message {
            Message::WillAppear {
                context, payload, ..
            } => {
                self.pool
                    .pin(&context, EndpointKey::from_settings(&payload.settings));
            }
            Message::DidReceiveSettings {
                context, payload, ..
            } => {
                self.pool
                    .pin(&context, EndpointKey::from_settings(&payload.settings));
                self.push_status(&context);
            }
            Message::WillDisappear { context, .. } => {
                self.pool.unpin(&context);
            }
            Message::KeyDown {
                action,
                context,
                payload,
                ..
            } => self.run_action(action, context, payload.settings, Gesture::Down),
            Message::KeyUp {
                action,
                context,
                payload,
                ..
            } => self.run_action(action, context, payload.settings, Gesture::Up),
            Message::PropertyInspectorDidAppear {
                action, context, ..
            } => {
                self.open_pi.insert(context.clone(), action);
                if self.pool.status_for_context(&context).is_none() {
                    let _ = self.outgoing.send(Outgoing::GetSettings {
                        context: context.clone(),
                    });
                }
                self.push_status(&context);
            }
            Message::PropertyInspectorDidDisappear { context, .. } => {
                self.open_pi.remove(&context);
            }
            Message::SendToPlugin {
                action,
                context,
                payload,
            } => self.on_pi_message(action, context, payload),
            _ => {}
        }
    }

    fn on_pi_message(&mut self, action: String, context: String, payload: PiMessage) {
        if payload.property_inspector.as_deref() == Some("propertyInspectorConnected") {
            self.open_pi.insert(context.clone(), action);
            self.push_status(&context);
            return;
        }
        if payload.command.as_deref() == Some("test_connection") {
            self.push_status(&context);
        }
    }

    fn on_status(&mut self, key: EndpointKey, status: ConnectionStatus) {
        self.pool.set_status(key.clone(), status.clone());
        for context in self.pool.contexts_for(&key) {
            self.push_status_value(&context, status.label());
        }
    }

    fn push_status(&self, context: &str) {
        let label = self
            .pool
            .status_for_context(context)
            .map(ConnectionStatus::label)
            .unwrap_or_else(|| "Not connected".to_string());
        self.push_status_value(context, label);
    }

    fn push_status_value(&self, context: &str, label: String) {
        let Some(action) = self.open_pi.get(context) else {
            return;
        };
        let _ = self.outgoing.send(Outgoing::ToPi {
            action: action.clone(),
            context: context.to_string(),
            payload: PiOut::status(label),
        });
    }

    fn run_action(
        &mut self,
        action: String,
        context: String,
        settings: ActionSettings,
        gesture: Gesture,
    ) {
        let job = match build_job(&action, &settings, gesture) {
            Ok(Some(job)) => job,
            Ok(None) => return,
            Err(e) => {
                let _ = self.outgoing.send(Outgoing::Log { message: e });
                if gesture == Gesture::Down {
                    let _ = self.outgoing.send(Outgoing::ShowAlert { context });
                }
                return;
            }
        };
        let Some(key) = EndpointKey::from_settings(&settings) else {
            if gesture == Gesture::Down {
                let _ = self.outgoing.send(Outgoing::ShowAlert { context });
            }
            return;
        };
        let tx = self.pool.sender(&key);
        let outgoing = self.outgoing.clone();
        let show_feedback = gesture == Gesture::Down || matches!(job, DeviceJob::Write(_));
        tokio::spawn(async move {
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx
                .send(Work::Exec {
                    job,
                    reply: reply_tx,
                })
                .is_err()
            {
                if show_feedback {
                    let _ = outgoing.send(Outgoing::ShowAlert { context });
                }
                return;
            }
            match reply_rx.await {
                Ok(Ok(())) => {
                    if show_feedback && gesture_show_ok(gesture) {
                        let _ = outgoing.send(Outgoing::ShowOk { context });
                    }
                }
                Ok(Err(e)) => {
                    let _ = outgoing.send(Outgoing::Log { message: e });
                    if show_feedback {
                        let _ = outgoing.send(Outgoing::ShowAlert { context });
                    }
                }
                Err(_) => {
                    if show_feedback {
                        let _ = outgoing.send(Outgoing::ShowAlert { context });
                    }
                }
            }
        });
    }
}

fn gesture_show_ok(gesture: Gesture) -> bool {
    gesture == Gesture::Down
}
