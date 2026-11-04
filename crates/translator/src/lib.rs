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
pub fn execute_emission(bytes: Vec<u8>, executor: &FileEmission) -> Result<Vec<u8>, TranslatorError> {
    let query = Signal::<emission_belief_nexus::Query>::from(bytes).restore().map_err(|error| TranslatorError::Signal(format!("{error:?}")))?;
    archive(&executor.execute(query))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPeer { pub peer_name: public::PeerName, pub caller: public::Caller }

/// PID, uid, gid and executable are facts observed from this connection.
pub fn observe_peer(stream: &UnixStream, contract_digest: public::ContractDigest) -> Result<public::Caller, TranslatorError> {
    let credentials = nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials)
        .map_err(|error| TranslatorError::Peer(error.to_string()))?;
    let pid = u32::try_from(credentials.pid()).map_err(|_| TranslatorError::Peer("peer pid is negative".into()))?;
    let executable = std::fs::read_link(format!("/proc/{pid}/exe")).and_then(std::fs::canonicalize).map_err(TranslatorError::Io)?;
    Ok(public::Caller {
        user_id: public::UserId(i64::from(credentials.uid())),
        group_id: public::GroupId(i64::from(credentials.gid())),
        process_id: public::ProcessId(i64::from(pid)),
        executable_path: public::ExecutablePath(executable.to_string_lossy().into_owned()),
        contract_digest,
    })
}

/// Durable authority is PeerName plus uid, gid and canonical executable.
/// ContractDigest establishes wire compatibility; ProcessId is never configured.
pub fn admit_peer(caller: public::Caller, peers: &public::PeerList) -> Result<AdmittedPeer, TranslatorError> {
    let matches = peers.0.iter().filter(|peer| {
        peer.user_id == caller.user_id && peer.group_id == caller.group_id
            && canonical_equal(&peer.executable_path.0, &caller.executable_path.0)
            && peer.contract_digest == caller.contract_digest
    }).collect::<Vec<_>>();
    match matches.as_slice() {
        [peer] => Ok(AdmittedPeer { peer_name: peer.peer_name.clone(), caller }),
        [] => Err(TranslatorError::Peer("peer is not configured".into())),
        _ => Err(TranslatorError::Peer("peer identity is ambiguous".into())),
    }
}

fn canonical_equal(configured: &str, observed: &str) -> bool {
    std::fs::canonicalize(configured).ok().zip(std::fs::canonicalize(observed).ok())
        .is_some_and(|(configured, observed)| configured == observed)
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
    pub fn revision(&self) -> Option<&meta::ConfigurationRevision> { self.revision.as_ref() }

    /// Apply only the authoritative Configured result. A skipped or repeated
    /// revision leaves the current configuration snapshot whole.
    pub fn apply(&mut self, configured: &meta::ConfiguredState) -> Result<(), TranslatorError> {
        let expected = self.revision.as_ref().map_or(0, |revision| revision.0 + 1);
        if configured.configuration_revision.0 != expected {
            return Err(TranslatorError::Configuration("non-coherent configuration revision".into()));
        }
        let mut candidate = self.clone();
        if configured.owner == meta::Owner::Translator {
            candidate.apply_set(configured.set.clone())?;
        }
        candidate.revision = Some(configured.configuration_revision.clone());
        *self = candidate;
        Ok(())
    }

    fn apply_set(&mut self, setting: meta::Set) -> Result<(), TranslatorError> {
        match setting {
            meta::Set::NexusSocket(value) => self.nexus_socket = Some(value),
            meta::Set::MetaNexusSocket(value) => self.meta_nexus_socket = Some(value),
            meta::Set::CallTimeout(value) if value.0 > 0 => self.call_timeout = Some(value),
            meta::Set::OutputLimit(value) if value.0 > 0 => self.output_limit = Some(value),
            meta::Set::TextBudget(value) if value.node_limit.0 > 0 && value.depth_limit.0 > 0 => self.text_budget = Some(value),
            meta::Set::TranslatorFrameCapacity(value) if value.0 > 0 => self.frame_capacity = Some(value),
            meta::Set::TranslatorTransportTimeout(value) if value.0 > 0 => self.transport_timeout = Some(value),
            meta::Set::AllowedNexusPeers(value) if !value.0.is_empty() => self.allowed_nexus_peers = Some(value),
            meta::Set::GitExecutable(value) if !value.0.is_empty() => self.git_executable = Some(value),
            _ => return Err(TranslatorError::Configuration("invalid translator-owned setting".into())),
        }
        Ok(())
    }

    pub fn ready(&self) -> bool {
        self.nexus_socket.is_some() && self.meta_nexus_socket.is_some() && self.call_timeout.is_some()
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
    let answer = forward_signal(Path::new(&target), archive(&meta::MetaCall { caller, query })?, capacity, timeout)?;
    let response = restore_meta_response(answer)?;
    if let meta::Response::Configured(state) = &response { configuration.apply(state)?; }
    stream.write_frame(&signal::FrameBody::from(encode_meta(&response).into_bytes()), capacity).map_err(signal_error)
}

/// Serve one binary emission request from the configured live Nexus peer.
pub fn serve_emission(
    stream: &mut UnixStream,
    configuration: &TranslatorConfiguration,
    expected_process: &meta::PairedProcess,
    contract_digest: public::ContractDigest,
    executor: &FileEmission,
) -> Result<(), TranslatorError> {
    use signal::{FrameReading, FrameWriting};
    let capacity = configuration.frame_capacity().ok_or_else(|| TranslatorError::Configuration("translator is awaiting frame capacity".into()))?;
    let caller = observe_peer(stream, contract_digest)?;
    if caller.user_id != expected_process.user_id || caller.group_id != expected_process.group_id || caller.process_id != expected_process.process_id {
        return Err(TranslatorError::Peer("caller is not the paired Nexus process".into()));
    }
    let peers = configuration.allowed_nexus_peers().ok_or_else(|| TranslatorError::Configuration("translator is awaiting allowed Nexus peers".into()))?;
    admit_peer(caller, peers)?;
    let request = Vec::<u8>::from(stream.read_frame(capacity).map_err(signal_error)?);
    let answer = execute_emission(request, executor)?;
    stream.write_frame(&signal::FrameBody::from(answer), capacity).map_err(signal_error)
}

pub fn owner(setting: &meta::Set) -> meta::Owner {
    match setting {
        meta::Set::NexusSocket(_) | meta::Set::MetaNexusSocket(_) | meta::Set::CallTimeout(_)
        | meta::Set::OutputLimit(_) | meta::Set::TextBudget(_) | meta::Set::TranslatorFrameCapacity(_)
        | meta::Set::TranslatorTransportTimeout(_) | meta::Set::AllowedNexusPeers(_) => meta::Owner::Translator,
        meta::Set::GitExecutable(_) => meta::Owner::Translator,
        _ => meta::Owner::Nexus,
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
    use std::path::PathBuf;

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
        let peers = public::PeerList(vec![public::PeerIdentity {
            peer_name: public::PeerName("test-peer".into()), user_id: caller.user_id.clone(),
            group_id: caller.group_id.clone(), executable_path: caller.executable_path.clone(), contract_digest: digest,
        }]);
        let admitted = admit_peer(caller.clone(), &peers).unwrap();
        assert_eq!(admitted.peer_name, public::PeerName("test-peer".into()));
        assert_eq!(admitted.caller.process_id, caller.process_id);
        let call = public_call(&admitted, public::Query::Subscribe);
        let restored = Signal::<public::Call>::from(archive_public_call(&call).unwrap()).restore().unwrap();
        assert_eq!(restored, call);
    }

    #[test]
    fn configuration_rejects_revision_gaps_without_mutation() {
        let mut configuration = TranslatorConfiguration::default();
        let gap = meta::ConfiguredState {
            set: meta::Set::CallTimeout(meta::Timeout(100)), owner: meta::Owner::Translator,
            configuration_revision: meta::ConfigurationRevision(2), readiness: meta::Readiness::Awaiting(vec![]),
        };
        assert!(configuration.apply(&gap).is_err());
        assert!(configuration.revision().is_none());
        let first = meta::ConfiguredState { configuration_revision: meta::ConfigurationRevision(0), ..gap };
        configuration.apply(&first).unwrap();
        assert_eq!(configuration.revision(), Some(&meta::ConfigurationRevision(0)));
    }

    #[test]
    fn emission_lane_is_binary_signal_only() {
        let executor = FileEmission::new(PathBuf::from("git")).unwrap();
        let query = emission_belief_nexus::Query::Snapshot(public::Repository(public::RepositoryPath("/definitely/missing".into())));
        let answer = execute_emission(archive(&query).unwrap(), &executor).unwrap();
        let response = Signal::<emission_belief_nexus::Response>::from(answer).restore().unwrap();
        assert!(matches!(response, emission_belief_nexus::Response::EmissionRefused(_)));
    }
}
