use meta_signal_belief_nexus::Enveloped;
use signal::{FrameReading, Restorable, Signal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = std::io::stdin();
    let envelope = input.read_frame(signal::FrameCapacity::default())?;
    meta_signal_belief_nexus::NexusBootstrap::admit(&Vec::<u8>::from(envelope))
        .map_err(|refusal| format!("bootstrap envelope refused: {refusal:?}"))?;
    let body = input.read_frame(signal::FrameCapacity::default())?;
    let bootstrap = Signal::<meta_signal_belief_nexus::NexusBootstrap>::from(Vec::<u8>::from(body))
        .restore()?;
    belief_nexus_daemon::server::run(bootstrap)?;
    Ok(())
}
