//! The bootstrap every Nexus starts with names a Place, an admission and the paired translator;
//! the Belief Nexus checks it before it has any effect, and keeps its memory at one path in the
//! Place.

use belief_nexus_daemon::bootstrap::{BootstrapRefusal, Owning, Validating};
use belief_nexus_wire::BeliefNexus;
use nexus_exchange::{
    Admission, FrameCapacityBytes, FrameDeadlineMilliseconds, MemoryPlacing, RetryDelayMilliseconds,
};
use nexus_serving::{InstanceId, NexusBootstrap};
use signal::{ByteViewable, Restorable, Signal, Signalizable};

const USER: i64 = 1000;

fn place() -> translator::Place {
    translator::Place {
        place_name: translator::PlaceName("belief".into()),
        runtime: translator::Runtime("/run/user/1000/nexus/belief".into()),
        state: translator::State("/home/u/.local/state/nexus/belief".into()),
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
        peer_identity: translator::PeerIdentity {
            user_id: translator::UserId(USER),
            process_id: translator::ProcessId(42),
        },
        instance_id: InstanceId("bootstrap".into()),
    }
}

#[test]
fn a_coherent_bootstrap_is_accepted_and_names_one_memory_path() {
    assert_eq!(bootstrap().validated(USER), Ok(()));
    assert_eq!(
        bootstrap().place.memory_path::<BeliefNexus>(),
        std::path::Path::new("/home/u/.local/state/nexus/belief/belief.memory")
    );
}

#[test]
fn the_owner_is_the_login_of_the_paired_translator() {
    assert_eq!(
        bootstrap().owner(),
        signal_belief_nexus::Owner::Login(translator::UserId(USER))
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
fn place_names_and_roots_that_are_unsafe_are_refused() {
    for name in ["", ".", "..", "a/b", "/abs"] {
        let mut value = bootstrap();
        value.place.place_name = translator::PlaceName(name.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnsafePlaceName(name.into())),
            "{name:?}"
        );
    }
    let mut value = bootstrap();
    value.place.state = translator::State("relative/state".into());
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
        value.place.runtime = translator::Runtime(path.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnnormalizedPlace(path.into())),
            "{path:?}"
        );
        let mut value = bootstrap();
        value.place.state = translator::State(path.into());
        assert_eq!(
            value.validated(USER),
            Err(BootstrapRefusal::UnnormalizedPlace(path.into())),
            "{path:?}"
        );
    }
    let mut value = bootstrap();
    value.place.runtime = translator::Runtime("relative/runtime".into());
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::RelativePlace("relative/runtime".into()))
    );
}

#[test]
fn the_paired_translator_must_be_the_effective_user() {
    assert_eq!(
        bootstrap().validated(USER + 1),
        Err(BootstrapRefusal::PairedUserNotEffectiveUser)
    );
    let mut value = bootstrap();
    value.peer_identity.user_id = translator::UserId(USER + 1);
    assert_eq!(
        value.validated(USER),
        Err(BootstrapRefusal::PairedUserNotEffectiveUser)
    );
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
