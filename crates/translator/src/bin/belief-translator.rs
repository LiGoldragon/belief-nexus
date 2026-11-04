use shared_translator::{Restorable, Signal};
use signal::FrameReading;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = std::io::stdin().read_frame(signal::FrameCapacity::default())?;
    let bootstrap = Signal::<meta_signal_belief_nexus::TranslatorBootstrap>::from(Vec::<u8>::from(body)).restore()?;
    nix::unistd::close(0)?;
    belief_nexus_translator::server::run(bootstrap)?;
    Ok(())
}
