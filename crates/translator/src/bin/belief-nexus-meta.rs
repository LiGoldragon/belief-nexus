use belief_nexus_wire::BeliefNexus;
use translator_client::{TextCommand, TextInvocation, TextLane};

fn main() -> std::process::ExitCode {
    TextInvocation::run::<BeliefNexus>(TextLane::MetaText, std::env::args().skip(1))
}
