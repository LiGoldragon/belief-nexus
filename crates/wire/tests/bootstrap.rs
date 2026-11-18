//! The Nexus bootstrap names a Place and a file, and is checked before it has any effect.

use belief_nexus_wire::bootstrap::{BootstrapRefusal, Component, Locating, Validating};
use belief_nexus_wire::transport::{MemoryFileName, NexusBootstrap};
use nexus_exchange::{
    Admission, FrameCapacityBytes, FrameDeadlineMilliseconds, RetryDelayMilliseconds,
};
use signal::{ByteViewable, Restorable, Signal, Signalizable};

const USER: i64 = 1000;

fn place() -> shared_translator::Place {
    shared_translator::Place {
        place_name: shared_translator::PlaceName("belief".into()),
        runtime: shared_translator::Runtime("/run/user/1000/nexus/belief".into()),
        state: shared_translator::State("/home/u/.local/state/nexus/belief".into()),
    }
}

fn admission(capacity: i64, deadline: i64) -> Admission {
    Admission {
        frame_deadline_milliseconds: FrameDeadlineMilliseconds(deadline),
        frame_capacity_bytes: FrameCapacityBytes(capacity),
        retry_delay_milliseconds: RetryDelayMilliseconds(25),
    }
}

fn bootstrap() -> NexusBootstrap {
    NexusBootstrap {
        place: place(),
        admission: admission(1_000_000, 1_000),
        peer_identity: shared_translator::PeerIdentity {
            user_id: shared_translator::UserId(USER),
            process_id: shared_translator::ProcessId(42),
        },
        owner: signal_belief_nexus::Owner::Login(shared_translator::UserId(USER)),
        memory_file_name: MemoryFileName("memory".into()),
    }
}

#[test]
fn a_coherent_bootstrap_is_accepted_and_names_one_memory_path() {
    assert_eq!(bootstrap().validated(USER), Ok(()));
    assert_eq!(
        bootstrap().memory_path(),
        std::path::Path::new("/home/u/.local/state/nexus/belief/memory")
    );
}

#[test]
fn a_bootstrap_restores_through_validated_signal() {
    let archived = bootstrap().signalize().expect("archive bootstrap");
    let restored = Signal::<NexusBootstrap>::from(archived.bytes().to_vec())
        .restore()
        .expect("restore");
    assert_eq!(restored, bootstrap());
}

#[test]
fn file_names_that_are_not_one_normal_component_are_refused() {
    for name in ["", ".", "..", "../memory", "a/b", "/abs", "mem\0ory"] {
        let mut value = bootstrap();
        value.memory_file_name = MemoryFileName(name.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnsafeFileName(name.into())),
            "{name:?}"
        );
    }
}

#[test]
fn a_file_name_that_is_its_own_legacy_name_is_refused_and_others_are_not() {
    for name in ["memory.sema", "belief.sema"] {
        let mut value = bootstrap();
        value.memory_file_name = MemoryFileName(name.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::LegacyExtension(name.into())),
            "{name:?}"
        );
    }
    for name in [
        "memory",
        "belief.memory",
        "sema",
        "a.sema.memory",
        "memory.SEMA",
    ] {
        let mut value = bootstrap();
        value.memory_file_name = MemoryFileName(name.into());
        assert_eq!(value.validated(USER), Ok(()), "{name:?}");
    }
}

#[test]
fn place_names_and_roots_that_are_unsafe_are_refused() {
    for name in ["", ".", "..", "a/b", "/abs"] {
        let mut value = bootstrap();
        value.place.place_name = shared_translator::PlaceName(name.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnsafePlaceName(name.into())),
            "{name:?}"
        );
    }
    let mut value = bootstrap();
    value.place.state = shared_translator::State("relative/state".into());
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::RelativePlace("relative/state".into()))
    );
    for path in [
        "/run/user/1000/nexus/belief/",
        "/run/user/1000//nexus/belief",
        "/run/user/1000/./nexus/belief",
        "/run/user/../1000/belief",
    ] {
        let mut value = bootstrap();
        value.place.runtime = shared_translator::Runtime(path.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnnormalizedPlace(path.into())),
            "{path:?}"
        );
        let mut value = bootstrap();
        value.place.state = shared_translator::State(path.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnnormalizedPlace(path.into())),
            "{path:?}"
        );
    }
    let mut value = bootstrap();
    value.place.runtime = shared_translator::Runtime("relative/runtime".into());
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::RelativePlace("relative/runtime".into()))
    );
}

#[test]
fn the_owner_and_the_paired_translator_must_be_the_effective_user() {
    assert_eq!(
        bootstrap().validated(USER + 1),
        Err(BootstrapRefusal::OwnerNotEffectiveUser)
    );
    let mut value = bootstrap();
    value.peer_identity.user_id = shared_translator::UserId(USER + 1);
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::PairedUserNotEffectiveUser)
    );
    let mut value = bootstrap();
    value.owner = signal_belief_nexus::Owner::Peer(shared_translator::PeerId("peer".into()));
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::OwnerNotEffectiveUser)
    );
}

#[test]
fn the_component_rule_is_exact() {
    for good in ["a", "ordinary.sock", ".hidden", "a b"] {
        assert!(good.is_one_component(), "{good:?}");
    }
    for bad in [
        "", ".", "..", "x/", "x/.", "/x", "a/b", "x\0y", "./x", "x//",
    ] {
        assert!(!bad.is_one_component(), "{bad:?}");
    }
}

#[test]
fn admission_limits_must_be_positive() {
    for (capacity, deadline) in [(0, 1_000), (-1, 1_000), (1_000, 0), (1_000, -5), (0, 0)] {
        let mut value = bootstrap();
        value.admission = admission(capacity, deadline);
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::InvalidAdmission),
            "{capacity} {deadline}"
        );
    }
}
