mod actions;
mod plugin_log;
mod pool;
mod settings;
mod tally;

use std::collections::HashMap;
use std::env;

use actions::{build_job, DeviceJob, Gesture, SELECT_PGM, SELECT_PST};
use futures::{SinkExt, StreamExt};
use pool::{ConnectionStatus, EndpointKey, Pool, Work};
use roland_rs::devices::v160hd::TallyState;
use settings::{ActionSettings, EndpointInfo, PiMessage, PiOut};
use streamdeck_rs::registration::RegistrationParams;
use streamdeck_rs::{ImagePayload, Message, MessageOut, StreamDeckSocket, Target};
use tally::{image_data_uri, TallyBinding, TallyLight};
use tokio::sync::{mpsc, oneshot};

/// Official `0C00xx` tally map: HDMI 00–07, SDI 08–0F, Still 10–1F, Input 20–33.
const TALLY_SLOTS: usize = 0x34;

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
    SetImage {
        context: String,
        image: Option<String>,
    },
    GetSettings {
        context: String,
    },
    Log {
        message: String,
    },
    Tested {
        action: String,
        context: String,
        host: String,
        password: String,
        ok: bool,
        status: String,
    },
}

struct KeyWatch {
    endpoint: Option<EndpointKey>,
    binding: TallyBinding,
}

struct Plugin {
    pool: Pool,
    open_pi: HashMap<String, String>,
    watches: HashMap<String, KeyWatch>,
    tally_states: HashMap<EndpointKey, [TallyState; TALLY_SLOTS]>,
    last_light: HashMap<String, Option<TallyLight>>,
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
    let (tally_tx, mut tally_rx) = mpsc::unbounded_channel();
    let (log_tx, mut log_rx) = mpsc::unbounded_channel();

    plugin_log::write_line(&format!("plugin start log={}", plugin_log::path_display()));

    let mut plugin = Plugin {
        pool: Pool::new(status_tx, idle_tx, tally_tx, log_tx),
        open_pi: HashMap::new(),
        watches: HashMap::new(),
        tally_states: HashMap::new(),
        last_light: HashMap::new(),
        outgoing: outgoing_tx,
    };

    loop {
        tokio::select! {
            msg = socket.next() => {
                match msg {
                    Some(Ok(message)) => plugin.handle(message),
                    Some(Err(e)) => {
                        plugin_log::write_line(&format!("streamdeck read error: {e:?}"));
                        if matches!(e, streamdeck_rs::socket::StreamDeckSocketError::WebSocketError(_)) {
                            break;
                        }
                    }
                    None => break,
                }
            }
            out = outgoing_rx.recv() => {
                let Some(out) = out else { break };
                if let Outgoing::Tested {
                    action,
                    context,
                    host,
                    password,
                    ok,
                    status,
                } = out
                {
                    if ok {
                        plugin.log(format!("test ok host={host} {status}"));
                        plugin.commit_connection(&context, host, password);
                    } else {
                        plugin.log(format!("test failed host={host} {status}"));
                    }
                    plugin.open_pi.insert(context.clone(), action.clone());
                    let _ = plugin.outgoing.send(Outgoing::ToPi {
                        action,
                        context,
                        payload: PiOut::test_result(status, ok),
                    });
                    continue;
                }
                if let Err(e) = send_out(&mut socket, out).await {
                    plugin_log::write_line(&format!("streamdeck write error: {e:?}"));
                    break;
                }
            }
            log = log_rx.recv() => {
                let Some(message) = log else { break };
                plugin.log(message);
            }
            status = status_rx.recv() => {
                let Some((key, status)) = status else { break };
                plugin.on_status(key, status);
            }
            idle = idle_rx.recv() => {
                let Some((key, generation)) = idle else { break };
                plugin.pool.apply_idle(key, generation);
            }
            tally = tally_rx.recv() => {
                let Some((key, updates)) = tally else { break };
                plugin.on_tally(key, updates);
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
        Outgoing::SetImage { context, image } => {
            socket
                .send(MessageOut::SetImage {
                    context,
                    payload: ImagePayload {
                        image,
                        target: Target::Both,
                        state: None,
                    },
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
        Outgoing::Tested { .. } => {
            unreachable!("Tested events are handled before send_out")
        }
    };
    result.map_err(|e| format!("{e:?}"))
}

impl Plugin {
    fn log(&self, message: impl Into<String>) {
        let message = message.into();
        plugin_log::write_line(&message);
        let _ = self.outgoing.send(Outgoing::Log {
            message: format!("[v160hd] {message}"),
        });
    }

    fn handle(&mut self, message: Message<(), ActionSettings, PiMessage>) {
        match message {
            Message::WillAppear {
                action,
                context,
                payload,
                ..
            } => {
                self.watch_key(action, context, payload.settings);
            }
            Message::DidReceiveSettings {
                action,
                context,
                payload,
                ..
            } => {
                self.watch_key(action, context.clone(), payload.settings);
                self.push_status(&context);
            }
            Message::WillDisappear { context, .. } => {
                self.unwatch_key(&context);
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
            self.start_ver_test(action, context, payload);
        }
    }

    fn start_ver_test(&mut self, action: String, context: String, payload: PiMessage) {
        let host = payload.host.unwrap_or_default();
        let host = host.trim().to_string();
        let password = payload.password.unwrap_or_else(|| "0000".to_string());
        if host.is_empty() {
            self.push_status_value(&context, "Enter a host".to_string());
            return;
        }
        self.log(format!(
            "test_connection host={host} password_len={}",
            password.len()
        ));
        self.push_status_value(&context, "Testing…".to_string());
        // V-160HD allows one TCP client. Never open a second probe socket.
        let key = EndpointKey::new(&host, &password);
        let tx = self.pool.sender(&key);
        self.log(format!("test_connection via pool host={host}"));
        let outgoing = self.outgoing.clone();
        tokio::spawn(async move {
            let (ok, status) = pool_probe(tx).await;
            let _ = outgoing.send(Outgoing::Tested {
                action,
                context,
                host,
                password,
                ok,
                status,
            });
        });
    }

    fn commit_connection(&mut self, context: &str, host: String, password: String) {
        let key = EndpointKey::new(host, password);
        if let Some(watch) = self.watches.get_mut(context) {
            watch.endpoint = Some(key.clone());
        }
        self.pool.pin(context, Some(key));
        self.refresh_tally_image(context);
    }

    fn on_tally(&mut self, key: EndpointKey, updates: Vec<(u8, TallyState)>) {
        let slot = self
            .tally_states
            .entry(key.clone())
            .or_insert([TallyState::Off; TALLY_SLOTS]);
        for (index, state) in updates {
            if let Some(entry) = slot.get_mut(index as usize) {
                *entry = state;
            }
        }
        let contexts = self.pool.contexts_for(&key);
        for context in contexts {
            self.refresh_tally_image(&context);
        }
    }

    fn watch_key(&mut self, action: String, context: String, settings: ActionSettings) {
        let endpoint = EndpointKey::from_settings(&settings);
        let binding = TallyBinding::from_action(&action, &settings);
        if settings.should_connect() {
            plugin_log::write_line(&format!(
                "pin host={} source={} tally={:?} connector={:?} watch={}",
                endpoint.as_ref().map(|k| k.host.as_str()).unwrap_or(""),
                settings.source,
                binding.check,
                binding.source,
                binding.watches_tally(),
            ));
            self.pool.pin(&context, endpoint.clone());
        }
        self.watches
            .insert(context.clone(), KeyWatch { endpoint, binding });
        self.refresh_tally_image(&context);
    }

    fn unwatch_key(&mut self, context: &str) {
        self.watches.remove(context);
        self.last_light.remove(context);
    }

    fn refresh_tally_image(&mut self, context: &str) {
        let Some(watch) = self.watches.get(context) else {
            return;
        };
        let light = match (
            watch.binding.watches_tally(),
            watch.binding.source,
            watch.endpoint.as_ref(),
        ) {
            (true, Some(index), Some(endpoint)) => self
                .tally_states
                .get(endpoint)
                .and_then(|states| states.get(index as usize).copied())
                .and_then(|state| watch.binding.check.light(state)),
            _ => None,
        };
        if self.last_light.get(context) == Some(&light) {
            return;
        }
        let previous = self.last_light.insert(context.to_string(), light);
        let image = match light {
            Some(light) => Some(image_data_uri(light)),
            None if previous.flatten().is_some() => Some(String::new()),
            None => return,
        };
        let _ = self.outgoing.send(Outgoing::SetImage {
            context: context.to_string(),
            image,
        });
    }

    fn on_status(&mut self, key: EndpointKey, status: ConnectionStatus) {
        self.pool.set_status(key.clone(), status.clone());
        if matches!(status, ConnectionStatus::Retrying { .. }) {
            self.tally_states.remove(&key);
            for context in self.pool.contexts_for(&key) {
                self.refresh_tally_image(&context);
            }
        }
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
        let endpoints = self
            .pool
            .endpoint_list()
            .into_iter()
            .map(|(key, status)| EndpointInfo {
                host: key.host,
                password: key.password,
                status,
            })
            .collect();
        let _ = self.outgoing.send(Outgoing::ToPi {
            action: action.clone(),
            context: context.to_string(),
            payload: PiOut::state(label, endpoints),
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
                self.log(format!("action error {action}: {e}"));
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
        let skip_ok = action == SELECT_PGM || action == SELECT_PST;
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
                    if show_feedback && gesture_show_ok(gesture) && !skip_ok {
                        let _ = outgoing.send(Outgoing::ShowOk { context });
                    }
                }
                Ok(Err(e)) => {
                    let _ = outgoing.send(Outgoing::Log {
                        message: format!("[v160hd] command error: {e}"),
                    });
                    plugin_log::write_line(&format!("command error: {e}"));
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

async fn pool_probe(tx: mpsc::UnboundedSender<Work>) -> (bool, String) {
    let (reply_tx, reply_rx) = oneshot::channel();
    if tx.send(Work::Probe { reply: reply_tx }).is_err() {
        return (false, "Failed (not connected)".to_string());
    }
    match reply_rx.await {
        Ok(Ok(())) => (true, "Connected".to_string()),
        Ok(Err(e)) => {
            plugin_log::write_line(&format!("pool_probe error={e}"));
            (false, format!("Failed ({e})"))
        }
        Err(_) => (false, "Failed (not connected)".to_string()),
    }
}
