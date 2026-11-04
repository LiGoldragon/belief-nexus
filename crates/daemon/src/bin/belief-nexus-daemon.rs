use signal::{FrameReading, Restorable, Signal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = std::io::stdin().read_frame(signal::FrameCapacity::default())?;
    let bootstrap = Signal::<meta_signal_belief_nexus::NexusBootstrap>::from(Vec::<u8>::from(body))
        .restore()?;
    belief_nexus_daemon::server::run(bootstrap)?;
    Ok(())
}
