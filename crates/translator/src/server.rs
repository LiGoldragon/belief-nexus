use crate::{meta_transport, observe_peer, serve_emission, serve_meta_text, serve_public_text, TranslatorConfiguration, TranslatorError};
use meta_signal_belief_nexus as meta;
use signal::Contracted as _;
use signal_belief_nexus as public;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

fn public_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", public::Query::contract_digest().0))
}
fn meta_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", meta::Query::contract_digest().0))
}
fn emission_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", emission_belief_nexus::Query::contract_digest().0))
}

struct GateState { configuration: TranslatorConfiguration, restored: bool }
struct ConfigurationGate { state: Mutex<GateState>, changed: Condvar }

pub fn run(bootstrap: meta::TranslatorBootstrap) -> Result<(), TranslatorError> {
    let ordinary_path = PathBuf::from(&((bootstrap.ordinary_translator_socket.0).0));
    let meta_path = PathBuf::from(&((bootstrap.meta_translator_socket.0).0));
    let ordinary = bind_new(&ordinary_path)?;
    let privileged = bind_new(&meta_path)?;
    let gate = Arc::new(ConfigurationGate { state: Mutex::new(GateState { configuration: TranslatorConfiguration::from_bootstrap(&bootstrap)?, restored: false }), changed: Condvar::new() });
    let ordinary_gate = Arc::clone(&gate);
    let paired = bootstrap.paired_process.clone();
    let bootstrap_capacity = signal::FrameCapacity::from(bootstrap.admission.byte_limit.0 as usize);
    std::thread::spawn(move || serve_ordinary_listener(ordinary, ordinary_gate, paired, bootstrap_capacity));
    let snapshot = restore_snapshot(&bootstrap)?;
    {
        let mut state = gate.state.lock().map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?;
        state.configuration.restore(&snapshot)?;
        state.restored = true;
        gate.changed.notify_all();
    }
    serve_meta(privileged, gate);
    Ok(())
}

fn serve_ordinary_listener(listener: UnixListener, gate: Arc<ConfigurationGate>, paired: meta::PairedProcess, capacity: signal::FrameCapacity) {
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let gate = Arc::clone(&gate);
        let paired = paired.clone();
        std::thread::spawn(move || {
            let result = serve_ordinary(&mut stream, &gate, &paired, capacity);
            if let Err(error) = result { let _ = write_failure(&mut stream, &error); }
        });
    }
}

fn serve_meta(listener: UnixListener, gate: Arc<ConfigurationGate>) {
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let gate = Arc::clone(&gate);
        std::thread::spawn(move || {
            let result = (|| {
                let mut state = gate.state.lock().map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?;
                while !state.restored { state = gate.changed.wait(state).map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?; }
                let result = serve_meta_text(&mut stream, &mut state.configuration, meta_digest());
                gate.changed.notify_all();
                result
            })();
            if let Err(error) = result { let _ = write_failure(&mut stream, &error); }
        });
    }
}

fn serve_ordinary(
    stream: &mut UnixStream,
    gate: &Arc<ConfigurationGate>,
    paired: &meta::PairedProcess,
    bootstrap_capacity: signal::FrameCapacity,
) -> Result<(), TranslatorError> {
    let credentials = shared_translator::unix::peer_credentials(stream).map_err(|error| TranslatorError::Peer(error.to_string()))?;
    if credentials.user_id == paired.user_id && credentials.group_id == paired.group_id && credentials.process_id == paired.process_id {
        serve_emission(stream, bootstrap_capacity, paired, emission_digest())
    } else {
        let mut state = gate.state.lock().map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?;
        while !state.restored || !state.configuration.ready() {
            state = gate.changed.wait(state).map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?;
        }
        let snapshot = state.configuration.clone();
        drop(state);
        serve_public_text(stream, &snapshot, public_digest())
    }
}

fn restore_snapshot(bootstrap: &meta::TranslatorBootstrap) -> Result<meta::TranslatorConfiguration, TranslatorError> {
    let target = Path::new(&((bootstrap.meta_nexus_socket_path.0).0));
    let capacity = signal::FrameCapacity::from(bootstrap.admission.byte_limit.0 as usize);
    let timeout = std::time::Duration::from_millis(bootstrap.admission.timeout.0 as u64);
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match meta_transport(target, meta::MetaTransportInput::RestoreTranslatorConfiguration, capacity, timeout) {
            Ok(meta::MetaTransportOutput::Restored(snapshot)) => return Ok(snapshot),
            Ok(_) => return Err(TranslatorError::Configuration("translator configuration restore refused".into())),
            Err(error) if std::time::Instant::now() < deadline => { let _ = error; std::thread::sleep(std::time::Duration::from_millis(10)); }
            Err(error) => return Err(error),
        }
    }
}

fn bind_new(path: &Path) -> Result<UnixListener, TranslatorError> {
    if path.exists() { return Err(TranslatorError::Configuration(format!("translator socket already exists: {}", path.display()))); }
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(TranslatorError::Io)?; }
    UnixListener::bind(path).map_err(TranslatorError::Io)
}

fn write_failure(stream: &mut UnixStream, error: &TranslatorError) -> Result<(), TranslatorError> {
    use signal::FrameWriting;
    stream.write_frame(&signal::FrameBody::from(error.to_string().into_bytes()), signal::FrameCapacity::default())
        .map_err(|error| TranslatorError::Signal(error.to_string()))
}
