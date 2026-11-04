use crate::{observe_peer, serve_emission, serve_meta_text, serve_public_text, TranslatorConfiguration, TranslatorError};
use emission_belief_nexus::FileEmission;
use meta_signal_belief_nexus as meta;
use signal::Contracted as _;
use signal_belief_nexus as public;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn public_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", public::Query::contract_digest()))
}
fn meta_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", meta::Query::contract_digest()))
}
fn emission_digest() -> public::ContractDigest {
    public::ContractDigest(format!("signal:{}", emission_belief_nexus::Query::contract_digest()))
}

pub fn run(bootstrap: meta::TranslatorBootstrap) -> Result<(), TranslatorError> {
    let ordinary_path = PathBuf::from(&((bootstrap.ordinary_translator_socket.0).0));
    let meta_path = PathBuf::from(&((bootstrap.meta_translator_socket.0).0));
    let ordinary = bind_new(&ordinary_path)?;
    let privileged = bind_new(&meta_path)?;
    let configuration = Arc::new(Mutex::new(TranslatorConfiguration::from_bootstrap(&bootstrap)?));
    let meta_configuration = Arc::clone(&configuration);
    std::thread::spawn(move || serve_meta(privileged, meta_configuration));
    for incoming in ordinary.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let configuration = Arc::clone(&configuration);
        let paired = bootstrap.paired_process.clone();
        std::thread::spawn(move || {
            let result = serve_ordinary(&mut stream, &configuration, &paired);
            if let Err(error) = result { let _ = write_failure(&mut stream, &error); }
        });
    }
    Ok(())
}

fn serve_meta(listener: UnixListener, configuration: Arc<Mutex<TranslatorConfiguration>>) {
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let configuration = Arc::clone(&configuration);
        std::thread::spawn(move || {
            let result = configuration.lock().map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))
                .and_then(|mut configuration| serve_meta_text(&mut stream, &mut configuration, meta_digest()));
            if let Err(error) = result { let _ = write_failure(&mut stream, &error); }
        });
    }
}

fn serve_ordinary(
    stream: &mut UnixStream,
    configuration: &Arc<Mutex<TranslatorConfiguration>>,
    paired: &meta::PairedProcess,
) -> Result<(), TranslatorError> {
    let caller = observe_peer(stream, emission_digest())?;
    let configuration = configuration.lock().map_err(|_| TranslatorError::Configuration("configuration lock poisoned".into()))?.clone();
    if caller.user_id == paired.user_id && caller.group_id == paired.group_id && caller.process_id == paired.process_id {
        let git = configuration.git_executable().ok_or_else(|| TranslatorError::Configuration("translator is awaiting Git executable".into()))?;
        let executor = FileEmission::new(PathBuf::from(git)).map_err(TranslatorError::Configuration)?;
        serve_emission(stream, &configuration, paired, emission_digest(), &executor)
    } else {
        serve_public_text(stream, &configuration, public_digest())
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
