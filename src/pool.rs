use std::collections::HashMap;
use std::time::{Duration, Instant};

use roland_rs::devices::v160hd;
use roland_rs::devices::v160hd::{TallySource, TallyState};
use roland_rs::{AsyncTelnetClient, Command, Response};
use tokio::sync::{mpsc, oneshot};

use crate::actions::DeviceJob;
use crate::settings::ActionSettings;

const IDLE_SECS: u64 = 30;
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const TALLY_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointKey {
    pub host: String,
    pub port: u16,
    pub password: String,
}

impl EndpointKey {
    pub fn new(host: impl Into<String>, password: impl Into<String>) -> Self {
        let password = password.into();
        Self {
            host: host.into(),
            port: v160hd::TELNET_PORT,
            password: if password.is_empty() {
                "0000".to_string()
            } else {
                password
            },
        }
    }

    pub fn from_settings(settings: &ActionSettings) -> Option<Self> {
        let host = settings.host_trimmed();
        if host.is_empty() {
            return None;
        }
        Some(Self::new(host, settings.password()))
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
    /// Succeeds once this endpoint has an authenticated TCP session.
    /// V-160HD accepts only one TCP client; Test must reuse this session.
    Probe {
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
    log_tx: mpsc::UnboundedSender<String>,
}

impl Pool {
    pub fn new(
        status_tx: mpsc::UnboundedSender<(EndpointKey, ConnectionStatus)>,
        idle_tx: mpsc::UnboundedSender<(EndpointKey, u64)>,
        tally_tx: mpsc::UnboundedSender<(EndpointKey, Vec<(u8, TallyState)>)>,
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            visible: HashMap::new(),
            endpoints: HashMap::new(),
            statuses: HashMap::new(),
            status_tx,
            idle_tx,
            tally_tx,
            log_tx,
        }
    }

    pub fn status_for_context(&self, context: &str) -> Option<&ConnectionStatus> {
        let key = self.visible.get(context)?;
        self.statuses.get(key)
    }

    pub fn set_status(&mut self, key: EndpointKey, status: ConnectionStatus) {
        self.statuses.insert(key, status);
    }

    pub fn endpoint_list(&self) -> Vec<(EndpointKey, String)> {
        let mut keys: Vec<EndpointKey> = self.endpoints.keys().cloned().collect();
        for key in self.visible.values() {
            if !keys.iter().any(|existing| existing == key) {
                keys.push(key.clone());
            }
        }
        keys.sort_by(|a, b| a.host.cmp(&b.host).then(a.password.cmp(&b.password)));
        keys.into_iter()
            .map(|key| {
                let status = self
                    .statuses
                    .get(&key)
                    .map(ConnectionStatus::label)
                    .unwrap_or_else(|| "Not connected".to_string());
                (key, status)
            })
            .collect()
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
        let log_tx = self.log_tx.clone();
        let task_key = key.clone();
        tokio::spawn(run_endpoint(task_key, rx, status_tx, tally_tx, log_tx));
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
    log_tx: mpsc::UnboundedSender<String>,
) {
    let mut client: Option<AsyncTelnetClient> = None;
    let mut backoff = Duration::from_secs(1);
    let mut last_tally: Option<Instant> = None;
    let mut next_tally_poll = tokio::time::Instant::now();
    let _ = log_tx.send(format!("pool start host={} port={}", key.host, key.port));

    loop {
        if client.is_none() {
            let _ = status_tx.send((key.clone(), ConnectionStatus::Connecting));
            let _ = log_tx.send(format!("tcp connect {}:{}", key.host, key.port));
            match AsyncTelnetClient::connect_v160hd(&key.host, &key.password).await {
                Ok(mut connected) => {
                    let _ = log_tx.send(format!("authenticated host={}", key.host));
                    // Companion sends DTH:0C0100 fire-and-forget. The unit often
                    // never ACKs this address; waiting for a reply closed the
                    // only allowed TCP session and blocked every key press.
                    match connected
                        .write_command(&v160hd::subscribe_tally(true))
                        .await
                    {
                        Ok(()) => {
                            let _ = log_tx.send(format!("subscribe_tally sent host={}", key.host));
                        }
                        Err(e) if is_hard_disconnect(&e) => {
                            let _ = log_tx
                                .send(format!("subscribe_tally failed host={}: {e}", key.host));
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
                        Err(e) => {
                            let _ = log_tx.send(format!(
                                "subscribe_tally write warning host={}: {e}",
                                key.host
                            ));
                        }
                    }
                    if request_tally_bytes(&mut connected, &key.host, &log_tx).await {
                        let status = ConnectionStatus::Retrying {
                            backoff_secs: backoff.as_secs().max(1),
                            error: "tally poll write failed".to_string(),
                        };
                        let _ = status_tx.send((key.clone(), status.clone()));
                        if wait_backoff(&mut rx, backoff, &status).await {
                            return;
                        }
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                    client = Some(connected);
                    last_tally = None;
                    next_tally_poll = tokio::time::Instant::now() + TALLY_POLL;
                    backoff = Duration::from_secs(1);
                    let _ = log_tx.send(format!("connected host={}", key.host));
                    let _ = status_tx.send((key.clone(), ConnectionStatus::Connected));
                }
                Err(e) => {
                    let _ = log_tx.send(format!("connect failed host={}: {e}", key.host));
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
                            let _ = log_tx.send(format!(
                                "recv host={} {}",
                                key.host,
                                format_response(&response)
                            ));
                            if let Some(updates) = v160hd::tally_updates(&response) {
                                last_tally = Some(Instant::now());
                                let _ = log_tx.send(format!(
                                    "tally host={} updates={}",
                                    key.host,
                                    updates.len()
                                ));
                                let _ = tally_tx.send((key.clone(), updates));
                            }
                        }
                        Err(e) => {
                            let _ = log_tx.send(format!("recv failed host={}: {e}", key.host));
                            if is_hard_disconnect(&e) {
                                drop_client = true;
                            }
                        }
                    }
                }
                work = rx.recv() => {
                    match work {
                        None | Some(Work::Stop) => {
                            let _ = log_tx.send(format!("pool stop host={}", key.host));
                            return;
                        }
                        Some(Work::Exec { job, reply }) => {
                            let result = execute(c, job).await;
                            if let Err(e) = &result {
                                let _ = log_tx.send(format!("command failed host={}: {e}", key.host));
                                if is_hard_disconnect(e) {
                                    drop_client = true;
                                }
                            }
                            let _ = reply.send(result.map_err(|e| e.to_string()));
                        }
                        Some(Work::Probe { reply }) => {
                            let _ = reply.send(Ok(()));
                        }
                    }
                }
                _ = tokio::time::sleep_until(next_tally_poll) => {
                    let stale = last_tally
                        .map(|t| t.elapsed() >= TALLY_POLL)
                        .unwrap_or(true);
                    if stale && request_tally_bytes(c, &key.host, &log_tx).await {
                        drop_client = true;
                    }
                    next_tally_poll = tokio::time::Instant::now() + TALLY_POLL;
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
                Some(Work::Probe { reply }) => {
                    let _ = reply.send(Err(status.label()));
                }
            }
        }
    }
}

fn is_hard_disconnect(err: &roland_rs::TelnetError) -> bool {
    match err {
        roland_rs::TelnetError::ConnectionClosed => true,
        roland_rs::TelnetError::Io(e) => matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}

fn format_response(response: &Response) -> String {
    match response {
        Response::Acknowledge => "ACK".to_string(),
        Response::Data { address, value } => {
            format!("DTH:{},{:02X}", address.to_hex(), value)
        }
        Response::DataBlock { address, bytes } => {
            let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            format!("DTH:{},{hex} ({}B)", address.to_hex(), bytes.len())
        }
        Response::Version { product, version } => format!("VER:{product}:{version}"),
        Response::Error(e) => format!("ERR:{e:?}"),
    }
}

fn tally_byte_reads() -> Vec<Command> {
    let mut commands = Vec::with_capacity(16);
    for n in 1..=8 {
        commands.push(v160hd::read_tally(TallySource::hdmi(n).expect("hdmi 1-8")));
    }
    for n in 1..=8 {
        commands.push(v160hd::read_tally(TallySource::sdi(n).expect("sdi 1-8")));
    }
    commands
}

/// Official `RQH:0C00xx,000001;` for each HDMI/SDI connector.
async fn request_tally_bytes(
    client: &mut AsyncTelnetClient,
    host: &str,
    log_tx: &mpsc::UnboundedSender<String>,
) -> bool {
    for command in tally_byte_reads() {
        match client.write_command(&command).await {
            Ok(()) => {}
            Err(e) if is_hard_disconnect(&e) => {
                let _ = log_tx.send(format!("tally poll failed host={host}: {e}"));
                return true;
            }
            Err(e) => {
                let _ = log_tx.send(format!("tally poll write warning host={host}: {e}"));
            }
        }
    }
    let _ = log_tx.send(format!("tally poll requested host={host} count=16"));
    false
}

async fn execute(
    client: &mut AsyncTelnetClient,
    job: DeviceJob,
) -> Result<(), roland_rs::TelnetError> {
    // Companion writes DTH fire-and-forget. Waiting 5s for ACK dropped the
    // only TCP session after every key press.
    match job {
        DeviceJob::Commands(commands) => {
            for command in &commands {
                client.write_command(command).await?;
            }
            Ok(())
        }
        DeviceJob::PressRelease(sw) => {
            client.write_command(&v160hd::press_switch(sw)).await?;
            tokio::time::sleep(Duration::from_millis(200)).await;
            client.write_command(&v160hd::release_switch(sw)).await
        }
        DeviceJob::Write(command) => client.write_command(&command).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ActionSettings;

    #[test]
    fn from_settings_skips_empty_host() {
        assert!(EndpointKey::from_settings(&ActionSettings::default()).is_none());
    }

    #[test]
    fn from_settings_uses_host_and_password() {
        let settings = ActionSettings {
            host: " 10.0.0.1 ".into(),
            password: "1234".into(),
            ..ActionSettings::default()
        };
        let key = EndpointKey::from_settings(&settings).unwrap();
        assert_eq!(key.host, "10.0.0.1");
        assert_eq!(key.password, "1234");
        assert_eq!(key.port, v160hd::TELNET_PORT);
    }

    #[test]
    fn timeout_is_not_a_hard_disconnect() {
        let err = roland_rs::TelnetError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "read timed out",
        ));
        assert!(!is_hard_disconnect(&err));
    }

    #[test]
    fn connection_reset_is_a_hard_disconnect() {
        let err = roland_rs::TelnetError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "reset",
        ));
        assert!(is_hard_disconnect(&err));
        assert!(is_hard_disconnect(
            &roland_rs::TelnetError::ConnectionClosed
        ));
    }

    #[test]
    fn tally_byte_reads_are_official_one_byte_rqh() {
        let cmds = tally_byte_reads();
        assert_eq!(cmds.len(), 16);
        assert_eq!(cmds[0].encode(), "RQH:0C0000,000001;");
        assert_eq!(cmds[7].encode(), "RQH:0C0007,000001;");
        assert_eq!(cmds[8].encode(), "RQH:0C0008,000001;");
        assert_eq!(cmds[15].encode(), "RQH:0C000F,000001;");
    }

    #[test]
    fn format_response_shows_tally_dth() {
        let response = Response::parse("DTH:0C0002,01;").unwrap();
        assert_eq!(format_response(&response), "DTH:0C0002,01");
    }
}
