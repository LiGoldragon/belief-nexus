use translator_serving::{PairReading, PairRunning, Pairing};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Pairing::from_arguments(std::env::args().skip(1))?.launch()? {}
}
