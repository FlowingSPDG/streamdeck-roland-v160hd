use std::collections::HashMap;
use std::time::Duration;

use roland_rs::devices::v160hd;
use roland_rs::devices::v160hd::TallyState;
use roland_rs::AsyncTelnetClient;
use tokio::sync::{mpsc, oneshot};

use crate::actions::DeviceJob;
use crate::settings::ActionSettings;

const IDLE_SECS: u64 = 30;
const MAX_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointKey {
    pub host: String,
    pub port: u16,
    pub password: String,
}

impl EndpointKey {
    pub fn from_settings(settings: &ActionSettings) -> Option<Self> {
        let host = settings.host_trimmed();
        if host.is_empty() {
            return None;
        }
        Some(Self {
            host: host.to_string(),
            port: v160hd::TELNET_PORT,
            password: settings.password().to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Retrying { backoff_secs: u64, error: String },
}

impl ConnectionStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Connecting => "Connecting…".to_string(),
            Self::Connected => "Connected".to_string(),
            Self::Retrying {
                backoff_secs,
                error,
            } => format!("Retrying in {backoff_secs}s ({error})"),
        }
    }
}

pub enum Work {
    Exec {
        job: DeviceJob,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Stop,
}

struct Slot {
    tx: mpsc::UnboundedSender<Work>,
    refs: usize,
    generation: u64,
}

pub struct Pool {
    visible: HashMap<String, EndpointKey>,
    endpoints: HashMap<EndpointKey, Slot>,
    statuses: HashMap<EndpointKey, ConnectionStatus>,
    status_tx: mpsc::UnboundedSender<(EndpointKey, ConnectionStatus)>,
    idle_tx: mpsc::UnboundedSender<(EndpointKey, u64)>,
    tally_tx: mpsc::UnboundedSender<(EndpointKey, Vec<(u8, TallyState)>)>,
}

impl Pool {
    pub fn new(
        status_tx: mpsc::UnboundedSender<(EndpointKey, ConnectionStatus)>,
        idle_tx: mpsc::UnboundedSender<(EndpointKey, u64)>,
        tally_tx: mpsc::UnboundedSender<(EndpointKey, Vec<(u8, TallyState)>)>,
    ) -> Self {
        Self {
            visible: HashMap::new(),
            endpoints: HashMap::new(),
            statuses: HashMap::new(),
            status_tx,
            idle_tx,
            tally_tx,
        }
    }

    pub fn status_for_context(&self, context: &str) -> Option<&ConnectionStatus> {
        let key = self.visible.get(context)?;
        self.statuses.get(key)
    }

    pub fn set_status(&mut self, key: EndpointKey, status: ConnectionStatus) {
        self.statuses.insert(key, status);
    }

    pub fn contexts_for(&self, key: &EndpointKey) -> Vec<String> {
        self.visible
            .iter()
            .filter_map(|(ctx, k)| if k == key { Some(ctx.clone()) } else { None })
            .collect()
    }

    pub fn pin(&mut self, context: &str, key: Option<EndpointKey>) {
        let previous = self.visible.remove(context);
        if previous.as_ref() == key.as_ref() {
            if let Some(existing) = key {
                self.visible.insert(context.to_string(), existing);
            }
            return;
        }
        if let Some(old) = previous {
            self.release_key(old);
        }
        if let Some(key) = key {
            self.ensure(key.clone());
            self.visible.insert(context.to_string(), key);
        }
    }

    pub fn unpin(&mut self, context: &str) {
        if let Some(key) = self.visible.remove(context) {
            self.release_key(key);
        }
    }

    fn ensure(&mut self, key: EndpointKey) {
        if let Some(slot) = self.endpoints.get_mut(&key) {
            slot.refs += 1;
            slot.generation = slot.generation.wrapping_add(1);
            return;
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let status_tx = self.status_tx.clone();
        let tally_tx = self.tally_tx.clone();
        let task_key = key.clone();
        tokio::spawn(run_endpoint(task_key, rx, status_tx, tally_tx));
        self.endpoints.insert(
            key,
            Slot {
                tx,
                refs: 1,
                generation: 0,
            },
        );
    }

    pub fn sender(&mut self, key: &EndpointKey) -> mpsc::UnboundedSender<Work> {
        if let Some(slot) = self.endpoints.get(key) {
            return slot.tx.clone();
        }
        self.ensure(key.clone());
        if let Some(slot) = self.endpoints.get_mut(key) {
            slot.refs = slot.refs.saturating_sub(1);
            if slot.refs == 0 {
                slot.generation = slot.generation.wrapping_add(1);
                let generation = slot.generation;
                let idle_tx = self.idle_tx.clone();
                let idle_key = key.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(IDLE_SECS)).await;
                    let _ = idle_tx.send((idle_key, generation));
                });
            }
            return slot.tx.clone();
        }
        unreachable!("endpoint spawned by ensure")
    }

    pub fn apply_idle(&mut self, key: EndpointKey, generation: u64) {
        let stop = self
            .endpoints
            .get(&key)
            .is_some_and(|slot| slot.refs == 0 && slot.generation == generation);
        if stop {
            if let Some(slot) = self.endpoints.remove(&key) {
                let _ = slot.tx.send(Work::Stop);
            }
            self.statuses.remove(&key);
        }
    }

    fn release_key(&mut self, key: EndpointKey) {
        let Some(slot) = self.endpoints.get_mut(&key) else {
            return;
        };
        slot.refs = slot.refs.saturating_sub(1);
        if slot.refs == 0 {
            slot.generation = slot.generation.wrapping_add(1);
            let generation = slot.generation;
            let idle_tx = self.idle_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(IDLE_SECS)).await;
                let _ = idle_tx.send((key, generation));
            });
        }
    }
}

async fn run_endpoint(
    key: EndpointKey,
    mut rx: mpsc::UnboundedReceiver<Work>,
    status_tx: mpsc::UnboundedSender<(EndpointKey, ConnectionStatus)>,
    tally_tx: mpsc::UnboundedSender<(EndpointKey, Vec<(u8, TallyState)>)>,
) {
    let mut client: Option<AsyncTelnetClient> = None;
    let mut backoff = Duration::from_secs(1);

    loop {
        if client.is_none() {
            let _ = status_tx.send((key.clone(), ConnectionStatus::Connecting));
            match AsyncTelnetClient::connect_v160hd(&key.host, &key.password).await {
                Ok(mut connected) => {
                    // Subscribe so the switcher pushes tally DTH on its own.
                    // recv() in the idle loop picks those up; no polling.
                    if let Err(e) = connected.send_command(&v160hd::subscribe_tally(true)).await {
                        let status = ConnectionStatus::Retrying {
                            backoff_secs: backoff.as_secs().max(1),
                            error: e.to_string(),
                        };
                        let _ = status_tx.send((key.clone(), status.clone()));
                        if wait_backoff(&mut rx, backoff, &status).await {
                            return;
                        }
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                    client = Some(connected);
                    backoff = Duration::from_secs(1);
                    let _ = status_tx.send((key.clone(), ConnectionStatus::Connected));
                }
                Err(e) => {
                    let status = ConnectionStatus::Retrying {
                        backoff_secs: backoff.as_secs().max(1),
                        error: e.to_string(),
                    };
                    let _ = status_tx.send((key.clone(), status.clone()));
                    if wait_backoff(&mut rx, backoff, &status).await {
                        return;
                    }
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }
            }
        }

        let mut drop_client = false;
        {
            let Some(c) = client.as_mut() else {
                continue;
            };
            tokio::select! {
                incoming = c.recv() => {
                    match incoming {
                        Ok(response) => {
                            if let Some(updates) = v160hd::tally_updates(&response) {
                                let _ = tally_tx.send((key.clone(), updates));
                            }
                        }
                        Err(_) => drop_client = true,
                    }
                }
                work = rx.recv() => match work {
                    None | Some(Work::Stop) => return,
                    Some(Work::Exec { job, reply }) => {
                        let result = execute(c, job).await;
                        if result.is_err() {
                            drop_client = true;
                        }
                        let _ = reply.send(result);
                    }
                }
            }
        }
        if drop_client {
            client = None;
        }
    }
}

async fn wait_backoff(
    rx: &mut mpsc::UnboundedReceiver<Work>,
    backoff: Duration,
    status: &ConnectionStatus,
) -> bool {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(backoff) => return false,
            work = rx.recv() => match work {
                None | Some(Work::Stop) => return true,
                Some(Work::Exec { reply, .. }) => {
                    let _ = reply.send(Err(status.label()));
                }
            }
        }
    }
}

async fn execute(client: &mut AsyncTelnetClient, job: DeviceJob) -> Result<(), String> {
    match job {
        DeviceJob::Commands(commands) => client
            .send_commands(&commands)
            .await
            .map_err(|e| e.to_string()),
        DeviceJob::PressRelease(sw) => client
            .press_and_release(sw)
            .await
            .map_err(|e| e.to_string()),
        DeviceJob::Write(command) => match client.send_command(&command).await {
            Ok(roland_rs::Response::Acknowledge) => Ok(()),
            Ok(roland_rs::Response::Error(e)) => Err(e.to_string()),
            Ok(_) => Err("unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        },
    }
}
