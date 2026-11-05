//! Text and external-effect boundaries for Belief Nexus.

pub mod server;

use datom_codec::{Actualizing, Budget, Potential};
use emission_belief_nexus::FileEmission;
use meta_signal_belief_nexus as meta;
use protos::ReaderBudget;
use shared_translator::{ByteViewable, Restorable, Signal};
use signal_belief_nexus as public;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextLimits { pub nodes: usize, pub depth: usize }

#[derive(datom_codec::Datomizable, datom_codec::Composing, Clone, Debug, Eq, PartialEq)]
pub struct NexusExecutable(pub String);
#[derive(datom_codec::Datomizable, datom_codec::Composing, Clone, Debug, Eq, PartialEq)]
pub struct TranslatorExecutable(pub String);
#[derive(datom_codec::Datomizable, datom_codec::Composing, Clone, Debug, Eq, PartialEq)]
pub struct SocketName(pub String);
#[derive(datom_codec::Datomizable, datom_codec::Composing, Clone, Debug, Eq, PartialEq)]
pub struct MemoryFileName(pub String);
#[derive(datom_codec::Datomizable, datom_codec::Composing, Clone, Debug, Eq, PartialEq)]
pub struct Deployment {
    pub nexus_executable: NexusExecutable,
    pub translator_executable: TranslatorExecutable,
    pub ordinary_socket_name: SocketName,
    pub meta_socket_name: SocketName,
    pub ordinary_translator_socket_name: SocketName,
    pub meta_translator_socket_name: SocketName,
    pub memory_file_name: MemoryFileName,
    pub admission: meta::Admission,
    pub owner: public::Owner,
    pub set_vector: Vec<meta::Set>,
}

pub fn deployment_from_text(text: &str) -> Result<Deployment, TranslatorError> {
    decode(text, TextLimits { nodes: 10_000, depth: 64 })
}

pub fn place_from_text(text: &str) -> Result<shared_translator::Place, TranslatorError> {
    decode(text, TextLimits { nodes: 100, depth: 8 })
}

pub fn decode_meta_response_text(text: &str) -> Result<meta::Response, TranslatorError> {
    decode(text, TextLimits { nodes: 10_000, depth: 64 })
}

impl TextLimits {
    pub fn validate(self) -> Result<Self, TranslatorError> {
        if self.nodes == 0 || self.depth == 0 || self.nodes > signal::MAXIMUM_SIGNAL_BYTES || self.depth > signal::MAXIMUM_SIGNAL_DEPTH {
            return Err(TranslatorError::Configuration("invalid text budget".into()));
        }
        Ok(self)
    }
    fn budget(self) -> Budget {
        Budget {
            remaining: i64::try_from(self.nodes).expect("validated node count fits i64"),
            reader: ReaderBudget { remaining: self.nodes },
            depth: 0,
            maximum_depth: i64::try_from(self.depth).expect("validated depth fits i64"),
        }
    }
}

pub fn decode_public(text: &str, limits: TextLimits) -> Result<public::Query, TranslatorError> { decode(text, limits) }
pub fn decode_meta(text: &str, limits: TextLimits) -> Result<meta::Query, TranslatorError> { decode(text, limits) }

fn decode<T: datom_codec::Composing>(text: &str, limits: TextLimits) -> Result<T, TranslatorError> {
    let mut budget = limits.validate()?.budget();
    Potential::<T>::from(text).actualize(&mut budget).map_err(|error| TranslatorError::Text(format!("{error:?}")))
}

pub fn encode_public_query(query: &public::Query) -> String { shared_translator::encode(query) }
pub fn encode_meta_query(query: &meta::Query) -> String { shared_translator::encode(query) }
pub fn encode_public(response: &public::Response) -> String { shared_translator::encode(response) }
pub fn encode_meta(response: &meta::Response) -> String { shared_translator::encode(response) }

/// A caller can only be constructed after the transport has observed and
/// admitted the live kernel peer. Text clients never supply this value.
pub fn public_call(peer: &AdmittedPeer, query: public::Query) -> public::Call {
    public::Call { caller: peer.caller.clone(), query }
}
pub fn meta_call(peer: &AdmittedPeer, query: meta::Query) -> meta::MetaCall {
    meta::MetaCall { caller: peer.caller.clone(), query }
}
pub fn archive_public_call(call: &public::Call) -> Result<Vec<u8>, TranslatorError> { archive(call) }
pub fn archive_meta_call(call: &meta::MetaCall) -> Result<Vec<u8>, TranslatorError> { archive(call) }

fn archive<T: signal::Signalizable>(value: &T) -> Result<Vec<u8>, TranslatorError> {
    value.signalize().map(|value| value.bytes().to_vec()).map_err(|error| TranslatorError::Signal(format!("{error:?}")))
}

pub fn restore_public_response(bytes: Vec<u8>) -> Result<public::Response, TranslatorError> {
    Signal::<public::Response>::from(bytes).restore().map_err(|error| TranslatorError::Signal(format!("{error:?}")))
}
pub fn restore_meta_response(bytes: Vec<u8>) -> Result<meta::Response, TranslatorError> {
    Signal::<meta::Response>::from(bytes).restore().map_err(|error| TranslatorError::Signal(format!("{error:?}")))
}

/// The emission lane is typed binary signal from end to end.
pub fn execute_emission(bytes: Vec<u8>, caller: &shared_translator::Caller) -> Result<Vec<u8>, TranslatorError> {
    let attempt = Signal::<emission_belief_nexus::EmissionAttempt>::from(bytes).restore().map_err(|error| TranslatorError::Signal(format!("{error:?}")))?;
    let configuration = TranslatorConfiguration::from_snapshot(&attempt.translator_configuration)?;
    if configuration.revision() != Some(&attempt.configuration_revision) {
        return Err(TranslatorError::Configuration("emission attempt revision does not match its snapshot".into()));
    }
    let peers = configuration.allowed_nexus_peers().ok_or_else(|| TranslatorError::Configuration("emission attempt is awaiting allowed Nexus peers".into()))?;
    admit_peer(caller.clone(), peers)?;
    let git = configuration.git_executable().ok_or_else(|| TranslatorError::Configuration("emission attempt is awaiting Git executable".into()))?;
    let executor = FileEmission::new(std::path::PathBuf::from(git)).map_err(TranslatorError::Configuration)?;
    let outcome = emission_belief_nexus::EmissionAttemptOutcome { configuration_revision: attempt.configuration_revision, response: executor.execute(attempt.query) };
    archive(&outcome)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPeer { pub peer_id: shared_translator::PeerId, pub caller: shared_translator::Caller }

/// PID, uid, gid and executable are facts observed from this connection.
pub fn observe_peer(stream: &UnixStream, _contract_digest: public::ContractDigest) -> Result<shared_translator::Caller, TranslatorError> {
    let credentials = shared_translator::unix::peer_credentials(stream).map_err(|error| TranslatorError::Peer(error.to_string()))?;
    Ok(shared_translator::Caller { process_id: credentials.process_id, user_id: credentials.user_id })
}

pub fn admit_peer(caller: shared_translator::Caller, peers: &public::PeerList) -> Result<AdmittedPeer, TranslatorError> {
    let matches = peers.0.iter().filter(|peer| peer.peer_identity.user_id == caller.user_id && peer.peer_identity.process_id == caller.process_id).collect::<Vec<_>>();
    match matches.as_slice() {
        [peer] => Ok(AdmittedPeer { peer_id: peer.peer_id.clone(), caller }),
        [] => Err(TranslatorError::Peer("peer is not configured".into())),
        _ => Err(TranslatorError::Peer("peer identity is ambiguous".into())),
    }
}

#[derive(Clone, Debug, Default)]
pub struct TranslatorConfiguration {
    revision: Option<meta::ConfigurationRevision>,
    nexus_socket: Option<meta::SocketPath>,
    meta_nexus_socket: Option<meta::SocketPath>,
    call_timeout: Option<meta::Timeout>,
    output_limit: Option<meta::ByteLimit>,
    text_budget: Option<meta::Budget>,
    frame_capacity: Option<meta::ByteLimit>,
    transport_timeout: Option<meta::Timeout>,
    allowed_nexus_peers: Option<public::PeerList>,
    git_executable: Option<public::ExecutablePath>,
    snapshot_ready: bool,
}

fn translator_set_missing(sets: &[meta::Set]) -> Vec<meta::SettingName> {
    let required = [meta::SettingName::NexusSocket, meta::SettingName::MetaNexusSocket, meta::SettingName::CallTimeout, meta::SettingName::OutputLimit, meta::SettingName::TextBudget, meta::SettingName::TranslatorFrameCapacity, meta::SettingName::TranslatorTransportTimeout, meta::SettingName::AllowedNexusPeers, meta::SettingName::GitExecutable];
    required.into_iter().filter(|name| !sets.iter().any(|set| setting_name_for_translator(set) == *name)).collect()
}
fn setting_name_for_translator(set: &meta::Set) -> meta::SettingName {
    match set {
        meta::Set::NexusSocket(_) => meta::SettingName::NexusSocket, meta::Set::MetaNexusSocket(_) => meta::SettingName::MetaNexusSocket,
        meta::Set::CallTimeout(_) => meta::SettingName::CallTimeout, meta::Set::OutputLimit(_) => meta::SettingName::OutputLimit,
        meta::Set::TextBudget(_) => meta::SettingName::TextBudget, meta::Set::TranslatorFrameCapacity(_) => meta::SettingName::TranslatorFrameCapacity,
        meta::Set::TranslatorTransportTimeout(_) => meta::SettingName::TranslatorTransportTimeout, meta::Set::AllowedNexusPeers(_) => meta::SettingName::AllowedNexusPeers,
        meta::Set::GitExecutable(_) => meta::SettingName::GitExecutable,
        _ => meta::SettingName::RepositoryPolling,
    }
}

impl TranslatorConfiguration {
    pub fn from_bootstrap(bootstrap: &meta::TranslatorBootstrap) -> Result<Self, TranslatorError> {
        let frame_capacity = usize::try_from(bootstrap.admission.byte_limit.0).ok()
            .filter(|value| *value > 0 && *value <= signal::MAXIMUM_SIGNAL_BYTES)
            .ok_or_else(|| TranslatorError::Configuration("invalid bootstrap frame capacity".into()))?;
        let timeout = bootstrap.admission.timeout.0;
        if timeout <= 0 { return Err(TranslatorError::Configuration("invalid bootstrap timeout".into())); }
        Ok(Self {
            nexus_socket: Some((bootstrap.ordinary_nexus_socket.0).clone()),
            meta_nexus_socket: Some((bootstrap.meta_nexus_socket_path.0).clone()),
            frame_capacity: Some(meta::ByteLimit(i64::try_from(frame_capacity).expect("Signal capacity fits i64"))),
            transport_timeout: Some(meta::Timeout(timeout)),
            ..Self::default()
        })
    }
    pub fn from_snapshot(snapshot: &meta::TranslatorConfiguration) -> Result<Self, TranslatorError> {
        if snapshot.configuration_revision.0 < 0 {
            return Err(TranslatorError::Configuration("negative configuration revision".into()));
        }
        let mut configuration = Self::default();
        let mut seen = std::collections::HashSet::new();
        for set in &snapshot.set_vector {
            if owner(set) != meta::Keeper::Translator {
                return Err(TranslatorError::Configuration("Nexus-owned setting in translator snapshot".into()));
            }
            let name = std::mem::discriminant(set);
            if !seen.insert(name) {
                return Err(TranslatorError::Configuration("duplicate translator setting".into()));
            }
            configuration.apply_set(set.clone())?;
        }
        configuration.revision = Some(snapshot.configuration_revision.clone());
        configuration.snapshot_ready = matches!(snapshot.readiness, meta::Readiness::Ready);
        let derived = if configuration.missing().is_empty() { meta::Readiness::Ready } else { meta::Readiness::Awaiting(configuration.missing()) };
        if derived != snapshot.readiness {
            return Err(TranslatorError::Configuration("translator readiness is not derived from its settings".into()));
        }
        Ok(configuration)
    }

    pub fn restore(&mut self, snapshot: &meta::TranslatorConfiguration) -> Result<(), TranslatorError> {
        if snapshot.configuration_revision.0 < 0 {
            return Err(TranslatorError::Configuration("negative configuration revision".into()));
        }
        if self.revision.as_ref().is_some_and(|current| snapshot.configuration_revision.0 < current.0) {
            return Err(TranslatorError::Configuration("stale configuration snapshot".into()));
        }
        let mut seen = std::collections::HashSet::new();
        let mut candidate = self.clone();
        for set in &snapshot.set_vector {
            if owner(set) != meta::Keeper::Translator || !seen.insert(std::mem::discriminant(set)) {
                return Err(TranslatorError::Configuration("invalid translator configuration snapshot".into()));
            }
            candidate.apply_set(set.clone())?;
        }
        candidate.revision = Some(snapshot.configuration_revision.clone());
        let stored_missing = translator_set_missing(&snapshot.set_vector);
        let derived = if stored_missing.is_empty() { meta::Readiness::Ready } else { meta::Readiness::Awaiting(stored_missing) };
        if derived != snapshot.readiness { return Err(TranslatorError::Configuration("translator readiness is not derived from its stored settings".into())); }
        candidate.snapshot_ready = matches!(snapshot.readiness, meta::Readiness::Ready);
        *self = candidate;
        Ok(())
    }

    pub fn revision(&self) -> Option<&meta::ConfigurationRevision> { self.revision.as_ref() }

    /// Apply only the authoritative Configured result. A skipped or repeated
    /// revision leaves the current configuration snapshot whole.
    pub fn apply(&mut self, configured: &meta::ConfiguredState) -> Result<(), TranslatorError> {
        if configured.keeper != owner(&configured.set) {
            return Err(TranslatorError::Configuration("configured keeper does not own its setting".into()));
        }
        let expected = match self.revision.as_ref() {
            Some(revision) => revision.0.checked_add(1).ok_or_else(|| {
                TranslatorError::Configuration("non-coherent configuration revision".into())
            })?,
            None => 0,
        };
        if configured.configuration_revision.0 != expected {
            return Err(TranslatorError::Configuration("non-coherent configuration revision".into()));
        }
        let mut candidate = self.clone();
        if configured.keeper == meta::Keeper::Translator {
            candidate.apply_set(configured.set.clone())?;
        }
        candidate.revision = Some(configured.configuration_revision.clone());
        candidate.snapshot_ready = matches!(configured.readiness, meta::Readiness::Ready);
        *self = candidate;
        Ok(())
    }

    fn apply_set(&mut self, setting: meta::Set) -> Result<(), TranslatorError> {
        match setting {
            meta::Set::NexusSocket(value) => self.nexus_socket = Some(value),
            meta::Set::MetaNexusSocket(value) => self.meta_nexus_socket = Some(value),
            meta::Set::CallTimeout(value) if value.0 > 0 => self.call_timeout = Some(value),
            meta::Set::OutputLimit(value) if value.0 > 0 => self.output_limit = Some(value),
            meta::Set::TextBudget(value) if usize::try_from(value.node_limit.0).ok().is_some_and(|nodes| nodes > 0 && nodes <= signal::MAXIMUM_SIGNAL_BYTES)
                && usize::try_from(value.depth_limit.0).ok().is_some_and(|depth| depth > 0 && depth <= signal::MAXIMUM_SIGNAL_DEPTH) => self.text_budget = Some(value),
            meta::Set::TranslatorFrameCapacity(value) if usize::try_from(value.0).ok().is_some_and(|capacity| capacity > 0 && capacity <= signal::MAXIMUM_SIGNAL_BYTES) => self.frame_capacity = Some(value),
            meta::Set::TranslatorTransportTimeout(value) if value.0 > 0 => self.transport_timeout = Some(value),
            meta::Set::AllowedNexusPeers(value) if !value.0.is_empty() => self.allowed_nexus_peers = Some(value),
            meta::Set::GitExecutable(value) if !value.0.is_empty() => self.git_executable = Some(value),
            _ => return Err(TranslatorError::Configuration("invalid translator-owned setting".into())),
        }
        Ok(())
    }

    fn missing(&self) -> Vec<meta::SettingName> {
        let mut missing = Vec::new();
        if self.nexus_socket.is_none() { missing.push(meta::SettingName::NexusSocket); }
        if self.meta_nexus_socket.is_none() { missing.push(meta::SettingName::MetaNexusSocket); }
        if self.call_timeout.is_none() { missing.push(meta::SettingName::CallTimeout); }
        if self.output_limit.is_none() { missing.push(meta::SettingName::OutputLimit); }
        if self.text_budget.is_none() { missing.push(meta::SettingName::TextBudget); }
        if self.frame_capacity.is_none() { missing.push(meta::SettingName::TranslatorFrameCapacity); }
        if self.transport_timeout.is_none() { missing.push(meta::SettingName::TranslatorTransportTimeout); }
        if self.allowed_nexus_peers.is_none() { missing.push(meta::SettingName::AllowedNexusPeers); }
        if self.git_executable.is_none() { missing.push(meta::SettingName::GitExecutable); }
        missing
    }

    pub fn ready(&self) -> bool {
        self.snapshot_ready && self.nexus_socket.is_some() && self.meta_nexus_socket.is_some() && self.call_timeout.is_some()
            && self.output_limit.is_some() && self.text_budget.is_some() && self.frame_capacity.is_some()
            && self.transport_timeout.is_some() && self.allowed_nexus_peers.is_some()
            && self.git_executable.is_some()
    }

    pub fn ordinary_nexus_socket(&self) -> Option<&str> { self.nexus_socket.as_ref().map(|value| value.0.as_str()) }
    pub fn meta_nexus_socket(&self) -> Option<&str> { self.meta_nexus_socket.as_ref().map(|value| value.0.as_str()) }
    pub fn text_limits(&self) -> Option<TextLimits> {
        self.text_budget.as_ref().and_then(|value| Some(TextLimits {
            nodes: usize::try_from(value.node_limit.0).ok()?,
            depth: usize::try_from(value.depth_limit.0).ok()?,
        }))
    }
    pub fn frame_capacity(&self) -> Option<signal::FrameCapacity> {
        self.frame_capacity.as_ref().and_then(|value| usize::try_from(value.0).ok()).map(signal::FrameCapacity::from)
    }
    pub fn transport_timeout(&self) -> Option<Duration> {
        self.transport_timeout.as_ref().and_then(|value| u64::try_from(value.0).ok()).map(Duration::from_millis)
    }
    pub fn allowed_nexus_peers(&self) -> Option<&public::PeerList> { self.allowed_nexus_peers.as_ref() }
    pub fn git_executable(&self) -> Option<&str> { self.git_executable.as_ref().map(|value| value.0.as_str()) }
}

/// A one-attempt client for the shared four-byte, big-endian Signal frame.
/// Text clients carry only the datom; caller identity is added by the
/// translator after observing this connection.
pub fn text_request(socket: &Path, text: &str, capacity: signal::FrameCapacity) -> Result<String, TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let mut stream = UnixStream::connect(socket).map_err(TranslatorError::Io)?;
    stream.write_frame(&signal::FrameBody::from(text.as_bytes().to_vec()), capacity).map_err(signal_error)?;
    let body = stream.read_frame(capacity).map_err(signal_error)?;
    String::from_utf8(Vec::<u8>::from(body)).map_err(|error| TranslatorError::Text(error.to_string()))
}

pub fn text_request_stream(
    socket: &Path,
    text: &str,
    capacity: signal::FrameCapacity,
    mut receive: impl FnMut(String) -> bool,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let mut stream = UnixStream::connect(socket).map_err(TranslatorError::Io)?;
    stream.write_frame(&signal::FrameBody::from(text.as_bytes().to_vec()), capacity).map_err(signal_error)?;
    loop {
        let body = match stream.read_frame(capacity) {
            Ok(body) => body,
            Err(signal::FrameError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(signal_error(error)),
        };
        let answer = String::from_utf8(Vec::<u8>::from(body)).map_err(|error| TranslatorError::Text(error.to_string()))?;
        if !receive(answer) { return Ok(()); }
    }
}

pub fn forward_signal(socket: &Path, request: Vec<u8>, capacity: signal::FrameCapacity, timeout: Duration) -> Result<Vec<u8>, TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let mut stream = UnixStream::connect(socket).map_err(TranslatorError::Io)?;
    stream.set_read_timeout(Some(timeout)).map_err(TranslatorError::Io)?;
    stream.set_write_timeout(Some(timeout)).map_err(TranslatorError::Io)?;
    stream.write_frame(&signal::FrameBody::from(request), capacity).map_err(signal_error)?;
    stream.read_frame(capacity).map(Vec::<u8>::from).map_err(signal_error)
}

fn signal_error(error: signal::FrameError) -> TranslatorError { TranslatorError::Signal(error.to_string()) }

/// Serve one ordinary text request. The supplied digest is the compatibility
/// digest of this translator's public contract, never caller authority.
pub fn serve_public_text(
    stream: &mut UnixStream,
    configuration: &TranslatorConfiguration,
    contract_digest: public::ContractDigest,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let capacity = configuration.frame_capacity().ok_or_else(|| TranslatorError::Configuration("translator is awaiting frame capacity".into()))?;
    let limits = configuration.text_limits().ok_or_else(|| TranslatorError::Configuration("translator is awaiting text budget".into()))?;
    let timeout = configuration.transport_timeout().ok_or_else(|| TranslatorError::Configuration("translator is awaiting transport timeout".into()))?;
    let target = configuration.ordinary_nexus_socket().ok_or_else(|| TranslatorError::Configuration("translator is awaiting ordinary Nexus socket".into()))?;
    let body = stream.read_frame(capacity).map_err(signal_error)?;
    let text = String::from_utf8(Vec::<u8>::from(body)).map_err(|error| TranslatorError::Text(error.to_string()))?;
    let query = decode_public(&text, limits)?;
    let subscription = matches!(query, public::Query::Subscribe);
    let caller = observe_peer(stream, contract_digest)?;
    let request = archive(&public::Call { caller, query })?;
    if subscription {
        return forward_public_stream(stream, Path::new(target), request, capacity, timeout);
    }
    let answer = forward_signal(Path::new(target), request, capacity, timeout)?;
    let response = restore_public_response(answer)?;
    stream.write_frame(&signal::FrameBody::from(encode_public(&response).into_bytes()), capacity).map_err(signal_error)
}

fn forward_public_stream(
    client: &mut UnixStream,
    target: &Path,
    request: Vec<u8>,
    capacity: signal::FrameCapacity,
    write_timeout: Duration,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let mut nexus = UnixStream::connect(target).map_err(TranslatorError::Io)?;
    nexus.set_write_timeout(Some(write_timeout)).map_err(TranslatorError::Io)?;
    nexus.write_frame(&signal::FrameBody::from(request), capacity).map_err(signal_error)?;
    client.set_write_timeout(Some(write_timeout)).map_err(TranslatorError::Io)?;
    loop {
        // A quiet subscription deliberately has no request/read idle timeout.
        let body = nexus.read_frame(capacity).map_err(signal_error)?;
        let response = restore_public_response(Vec::<u8>::from(body))?;
        let terminal = matches!(response, public::Response::SubscriptionClosed(_));
        client.write_frame(&signal::FrameBody::from(encode_public(&response).into_bytes()), capacity).map_err(signal_error)?;
        if terminal { return Ok(()); }
    }
}

/// Serve one meta text request and apply an acknowledged translator-owned
/// configuration change as one coherent revision.
pub fn serve_meta_text(
    stream: &mut UnixStream,
    configuration: &mut TranslatorConfiguration,
    contract_digest: public::ContractDigest,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let capacity = configuration.frame_capacity().unwrap_or_default();
    let limits = configuration.text_limits().unwrap_or(TextLimits { nodes: signal::MAXIMUM_SIGNAL_BYTES, depth: signal::MAXIMUM_SIGNAL_DEPTH });
    let timeout = configuration.transport_timeout().unwrap_or(Duration::from_secs(5));
    let target = configuration.meta_nexus_socket().ok_or_else(|| TranslatorError::Configuration("translator is awaiting meta Nexus socket".into()))?.to_owned();
    let body = stream.read_frame(capacity).map_err(signal_error)?;
    let text = String::from_utf8(Vec::<u8>::from(body)).map_err(|error| TranslatorError::Text(error.to_string()))?;
    let query = decode_meta(&text, limits)?;
    let caller = observe_peer(stream, contract_digest)?;
    let output = meta_transport(Path::new(&target), meta::MetaTransportInput::Call(meta::MetaCall { caller, query }), capacity, timeout)?;
    let mut response = match output {
        meta::MetaTransportOutput::Answered(response) => response,
        _ => return Err(TranslatorError::Signal("unexpected meta transport output".into())),
    };
    if let meta::Response::Configured(state) = &response {
        if configuration.apply(state).is_err() {
            match meta_transport(Path::new(&target), meta::MetaTransportInput::RestoreTranslatorConfiguration, capacity, timeout)? {
                meta::MetaTransportOutput::Restored(snapshot) => configuration.restore(&snapshot)?,
                _ => return Err(TranslatorError::Configuration("translator configuration restore refused".into())),
            }
            response = meta::Response::ConfigureRefused(meta::ConfigurationRefusal::Unavailable);
        }
    }
    stream.write_frame(&signal::FrameBody::from(encode_meta(&response).into_bytes()), capacity).map_err(signal_error)
}

pub fn meta_transport(
    socket: &Path,
    input: meta::MetaTransportInput,
    capacity: signal::FrameCapacity,
    timeout: Duration,
) -> Result<meta::MetaTransportOutput, TranslatorError> {
    let bytes = archive(&input)?;
    let answer = forward_signal(socket, bytes, capacity, timeout)?;
    Signal::<meta::MetaTransportOutput>::from(answer).restore().map_err(|error| TranslatorError::Signal(format!("{error:?}")))
}

/// Serve one binary emission request from the configured live Nexus peer.
pub fn serve_emission(
    stream: &mut UnixStream,
    capacity: signal::FrameCapacity,
    expected_process: &meta::PairedProcess,
    contract_digest: public::ContractDigest,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let credentials = shared_translator::unix::peer_credentials(stream).map_err(|error| TranslatorError::Peer(error.to_string()))?;
    let caller = shared_translator::Caller { process_id: credentials.process_id.clone(), user_id: credentials.user_id.clone() };
    if credentials.user_id != expected_process.user_id || credentials.group_id != expected_process.group_id || credentials.process_id != expected_process.process_id {
        return Err(TranslatorError::Peer("caller is not the paired Nexus process".into()));
    }
    let request = Vec::<u8>::from(stream.read_frame(capacity).map_err(signal_error)?);
    let answer = execute_emission(request, &caller)?;
    stream.write_frame(&signal::FrameBody::from(answer), capacity).map_err(signal_error)
}

pub fn owner(setting: &meta::Set) -> meta::Keeper {
    match setting {
        meta::Set::NexusSocket(_) | meta::Set::MetaNexusSocket(_) | meta::Set::CallTimeout(_)
        | meta::Set::OutputLimit(_) | meta::Set::TextBudget(_) | meta::Set::TranslatorFrameCapacity(_)
        | meta::Set::TranslatorTransportTimeout(_) | meta::Set::AllowedNexusPeers(_) => meta::Keeper::Translator,
        meta::Set::GitExecutable(_) => meta::Keeper::Translator,
        _ => meta::Keeper::Nexus,
    }
}

#[derive(Debug)]
pub enum TranslatorError { Text(String), Signal(String), Peer(String), Configuration(String), Io(std::io::Error) }
impl std::fmt::Display for TranslatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(formatter, "belief translator: {self:?}") }
}
impl std::error::Error for TranslatorError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_snapshot(revision: i64, caller: &shared_translator::Caller) -> meta::TranslatorConfiguration {
        let peer = shared_translator::PeerBinding { peer_id: shared_translator::PeerId("nexus".into()), peer_identity: shared_translator::PeerIdentity { user_id: caller.user_id.clone(), process_id: caller.process_id.clone() } };
        meta::TranslatorConfiguration {
            configuration_revision: meta::ConfigurationRevision(revision),
            set_vector: vec![
                meta::Set::NexusSocket(meta::SocketPath("ordinary".into())),
                meta::Set::MetaNexusSocket(meta::SocketPath("meta".into())),
                meta::Set::CallTimeout(meta::Timeout(100)),
                meta::Set::OutputLimit(meta::ByteLimit(1000)),
                meta::Set::TextBudget(meta::Budget { node_limit: meta::NodeLimit(1000), depth_limit: meta::DepthLimit(64) }),
                meta::Set::TranslatorFrameCapacity(meta::ByteLimit(1000)),
                meta::Set::TranslatorTransportTimeout(meta::Timeout(100)),
                meta::Set::AllowedNexusPeers(public::PeerList(vec![peer])),
                meta::Set::GitExecutable(public::ExecutablePath("git".into())),
            ],
            readiness: meta::Readiness::Ready,
        }
    }

    fn current_caller() -> shared_translator::Caller {
        shared_translator::Caller {
            user_id: shared_translator::UserId(i64::from(nix::unistd::getuid().as_raw())),
            process_id: shared_translator::ProcessId(i64::from(std::process::id())),
        }
    }

    fn assert_same_configuration(actual: &TranslatorConfiguration, expected: &TranslatorConfiguration) {
        assert_eq!(actual.revision, expected.revision);
        assert_eq!(actual.nexus_socket, expected.nexus_socket);
        assert_eq!(actual.meta_nexus_socket, expected.meta_nexus_socket);
        assert_eq!(actual.call_timeout, expected.call_timeout);
        assert_eq!(actual.output_limit, expected.output_limit);
        assert_eq!(actual.text_budget, expected.text_budget);
        assert_eq!(actual.frame_capacity, expected.frame_capacity);
        assert_eq!(actual.transport_timeout, expected.transport_timeout);
        assert_eq!(actual.allowed_nexus_peers, expected.allowed_nexus_peers);
        assert_eq!(actual.git_executable, expected.git_executable);
        assert_eq!(actual.snapshot_ready, expected.snapshot_ready);
    }

    #[test]
    fn ordinary_and_meta_text_round_trip() {
        let limits = TextLimits { nodes: 4_096, depth: 64 };
        let ordinary = public::Query::Resolve(vec![public::ModuleName { kind: public::Kind::Vision, subject: public::Subject("nexus".into()) }]);
        assert_eq!(decode_public(&shared_translator::encode(&ordinary), limits).unwrap(), ordinary);
        let privileged = meta::Query::Configure(meta::ConfigureRequest {
            configuration_revision_option: Some(meta::ConfigurationRevision(4)),
            set: meta::Set::SubscriberCapacity(8),
        });
        assert_eq!(decode_meta(&shared_translator::encode(&privileged), limits).unwrap(), privileged);
    }

    #[test]
    fn kernel_peer_pid_is_observed_and_not_configured() {
        let (left, _right) = UnixStream::pair().unwrap();
        let digest = public::ContractDigest("sha256:belief".into());
        let caller = observe_peer(&left, digest.clone()).unwrap();
        assert!(caller.process_id.0 > 0);
        let peers = public::PeerList(vec![shared_translator::PeerBinding { peer_id: shared_translator::PeerId("test-peer".into()), peer_identity: shared_translator::PeerIdentity { user_id: caller.user_id.clone(), process_id: caller.process_id.clone() } }]);
        let admitted = admit_peer(caller.clone(), &peers).unwrap();
        assert_eq!(admitted.peer_id, shared_translator::PeerId("test-peer".into()));
        assert_eq!(admitted.caller.process_id, caller.process_id);
        let call = public_call(&admitted, public::Query::Subscribe);
        let restored = Signal::<public::Call>::from(archive_public_call(&call).unwrap()).restore().unwrap();
        assert_eq!(restored, call);
    }

    #[test]
    fn configuration_rejects_revision_gaps_without_mutation() {
        let mut configuration = TranslatorConfiguration::default();
        let gap = meta::ConfiguredState {
            set: meta::Set::CallTimeout(meta::Timeout(100)), keeper: meta::Keeper::Translator,
            configuration_revision: meta::ConfigurationRevision(2), readiness: meta::Readiness::Awaiting(vec![]),
        };
        assert!(configuration.apply(&gap).is_err());
        assert!(configuration.revision().is_none());
        let first = meta::ConfiguredState { configuration_revision: meta::ConfigurationRevision(0), ..gap };
        configuration.apply(&first).unwrap();
        assert_eq!(configuration.revision(), Some(&meta::ConfigurationRevision(0)));
    }

    #[test]
    fn configuration_refuses_exhausted_revision_without_mutation() {
        let caller = current_caller();
        let mut configuration =
            TranslatorConfiguration::from_snapshot(&complete_snapshot(i64::MAX, &caller)).unwrap();
        let exhausted = meta::ConfiguredState {
            set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
            keeper: meta::Keeper::Translator,
            configuration_revision: meta::ConfigurationRevision(i64::MIN),
            readiness: meta::Readiness::Ready,
        };

        assert!(configuration.apply(&exhausted).is_err());
        assert_eq!(configuration.revision(), Some(&meta::ConfigurationRevision(i64::MAX)));
        assert_eq!(configuration.ordinary_nexus_socket(), Some("ordinary"));
        assert!(configuration.ready());
    }

    #[test]
    fn configured_keeper_must_own_its_setting_before_any_state_changes() {
        let caller = current_caller();
        let initial = TranslatorConfiguration::from_snapshot(&complete_snapshot(7, &caller)).unwrap();
        for mismatched in [
            meta::ConfiguredState {
                set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
                keeper: meta::Keeper::Nexus,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Awaiting(vec![]),
            },
            meta::ConfiguredState {
                set: meta::Set::RepositoryPolling(meta::Timeout(250)),
                keeper: meta::Keeper::Translator,
                configuration_revision: meta::ConfigurationRevision(8),
                readiness: meta::Readiness::Awaiting(vec![]),
            },
        ] {
            let mut configuration = initial.clone();
            assert!(matches!(configuration.apply(&mismatched), Err(TranslatorError::Configuration(_))));
            assert_same_configuration(&configuration, &initial);
        }

        let mut translator_owned = initial.clone();
        translator_owned.apply(&meta::ConfiguredState {
            set: meta::Set::NexusSocket(meta::SocketPath("changed".into())),
            keeper: meta::Keeper::Translator,
            configuration_revision: meta::ConfigurationRevision(8),
            readiness: meta::Readiness::Ready,
        }).unwrap();
        assert_eq!(translator_owned.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(translator_owned.ordinary_nexus_socket(), Some("changed"));
        assert!(translator_owned.ready());

        let mut nexus_owned = initial;
        nexus_owned.apply(&meta::ConfiguredState {
            set: meta::Set::RepositoryPolling(meta::Timeout(250)),
            keeper: meta::Keeper::Nexus,
            configuration_revision: meta::ConfigurationRevision(8),
            readiness: meta::Readiness::Awaiting(vec![meta::SettingName::RepositoryPolling]),
        }).unwrap();
        assert_eq!(nexus_owned.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(nexus_owned.ordinary_nexus_socket(), Some("ordinary"));
        assert!(!nexus_owned.ready());
    }

    #[test]
    fn snapshots_refuse_negative_revisions_before_construction_or_restore_mutation() {
        let caller = current_caller();
        for invalid in [-1, i64::MIN] {
            let mut snapshot = complete_snapshot(invalid, &caller);
            snapshot.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("invalid".into()));
            assert!(matches!(TranslatorConfiguration::from_snapshot(&snapshot), Err(TranslatorError::Configuration(_))));

            let mut current = TranslatorConfiguration::from_snapshot(&complete_snapshot(8, &caller)).unwrap();
            let before = current.clone();
            assert!(matches!(current.restore(&snapshot), Err(TranslatorError::Configuration(_))));
            assert_eq!(current.revision, before.revision);
            assert_eq!(current.nexus_socket, before.nexus_socket);
            assert_eq!(current.meta_nexus_socket, before.meta_nexus_socket);
            assert_eq!(current.call_timeout, before.call_timeout);
            assert_eq!(current.output_limit, before.output_limit);
            assert_eq!(current.text_budget, before.text_budget);
            assert_eq!(current.frame_capacity, before.frame_capacity);
            assert_eq!(current.transport_timeout, before.transport_timeout);
            assert_eq!(current.allowed_nexus_peers, before.allowed_nexus_peers);
            assert_eq!(current.git_executable, before.git_executable);
            assert_eq!(current.snapshot_ready, before.snapshot_ready);
        }

        let zero = complete_snapshot(0, &caller);
        assert_eq!(TranslatorConfiguration::from_snapshot(&zero).unwrap().revision(), Some(&meta::ConfigurationRevision(0)));
        let mut restored = TranslatorConfiguration::default();
        restored.restore(&zero).unwrap();
        assert_eq!(restored.revision(), Some(&meta::ConfigurationRevision(0)));
        assert!(restored.ready());
    }

    #[test]
    fn restore_refuses_stale_snapshots_and_reconciles_equal_or_newer_revisions() {
        let caller = current_caller();
        let initial = TranslatorConfiguration::from_snapshot(&complete_snapshot(8, &caller)).unwrap();

        let mut stale = complete_snapshot(7, &caller);
        stale.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("stale".into()));
        let mut current = initial.clone();
        assert!(matches!(current.restore(&stale), Err(TranslatorError::Configuration(_))));
        assert_same_configuration(&current, &initial);

        let mut equal = complete_snapshot(8, &caller);
        equal.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("equal".into()));
        current.restore(&equal).unwrap();
        assert_eq!(current.revision(), Some(&meta::ConfigurationRevision(8)));
        assert_eq!(current.ordinary_nexus_socket(), Some("equal"));
        assert!(current.ready());

        let mut newer = complete_snapshot(10, &caller);
        newer.set_vector[0] = meta::Set::NexusSocket(meta::SocketPath("newer".into()));
        current.restore(&newer).unwrap();
        assert_eq!(current.revision(), Some(&meta::ConfigurationRevision(10)));
        assert_eq!(current.ordinary_nexus_socket(), Some("newer"));
        assert!(current.ready());
    }

    #[test]
    fn emission_lane_is_binary_signal_only() {
        let caller = current_caller();
        let revision = meta::ConfigurationRevision(1);
        let query = emission_belief_nexus::Query::Snapshot(public::Repository(public::RepositoryPath("/definitely/missing".into())));
        let attempt = emission_belief_nexus::EmissionAttempt { configuration_revision: revision, translator_configuration: complete_snapshot(1, &caller), query };
        let answer = execute_emission(archive(&attempt).unwrap(), &caller).unwrap();
        let outcome = Signal::<emission_belief_nexus::EmissionAttemptOutcome>::from(answer).restore().unwrap();
        assert!(matches!(outcome.response, emission_belief_nexus::Response::EmissionRefused(_)));
    }

    #[test]
    fn emission_outcome_carries_the_attempt_revision() {
        let caller = current_caller();
        let query = emission_belief_nexus::Query::Snapshot(public::Repository(public::RepositoryPath("/definitely/missing".into())));
        let attempt = emission_belief_nexus::EmissionAttempt {
            configuration_revision: meta::ConfigurationRevision(7),
            translator_configuration: complete_snapshot(7, &caller), query,
        };
        let answer = execute_emission(archive(&attempt).unwrap(), &caller).unwrap();
        let outcome = Signal::<emission_belief_nexus::EmissionAttemptOutcome>::from(answer).restore().unwrap();
        assert_eq!(outcome.configuration_revision, meta::ConfigurationRevision(7));
    }

    #[test]
    fn snapshot_rejects_duplicate_nexus_owned_and_false_readiness() {
        let caller = current_caller();
        let mut duplicate = complete_snapshot(1, &caller);
        duplicate.set_vector.push(meta::Set::CallTimeout(meta::Timeout(200)));
        assert!(TranslatorConfiguration::from_snapshot(&duplicate).is_err());

        let mut nexus_owned = complete_snapshot(1, &caller);
        nexus_owned.set_vector.push(meta::Set::RepositoryPolling(meta::Timeout(10)));
        assert!(TranslatorConfiguration::from_snapshot(&nexus_owned).is_err());

        let mut false_readiness = complete_snapshot(1, &caller);
        false_readiness.set_vector.pop();
        assert!(TranslatorConfiguration::from_snapshot(&false_readiness).is_err());
    }

    #[test]
    fn deployment_text_round_trips() {
        let deployment = Deployment {
            nexus_executable: NexusExecutable("/nexus".into()),
            translator_executable: TranslatorExecutable("/translator".into()),
            ordinary_socket_name: SocketName("belief.sock".into()),
            meta_socket_name: SocketName("belief-meta.sock".into()),
            ordinary_translator_socket_name: SocketName("belief-translator.sock".into()),
            meta_translator_socket_name: SocketName("belief-meta-translator.sock".into()),
            memory_file_name: MemoryFileName("belief.memory".into()),
            admission: meta::Admission { byte_limit: meta::ByteLimit(8_388_608), timeout: meta::Timeout(70_000) },
            owner: public::Owner::Login(shared_translator::UserId(1000)),
            set_vector: vec![],
        };
        let text = shared_translator::encode(&deployment);
        eprintln!("deployment datom: {text}");
        assert_eq!(deployment_from_text(&text).unwrap(), deployment);
    }
}
