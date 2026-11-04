fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let metaflow = arguments.next().ok_or("METAFLOW")?;
    let datom = arguments.next().ok_or("DATOM")?;
    let socket_name = arguments.next().ok_or("TRANSLATOR_SOCKET_NAME")?;
    if arguments.next().is_some() { return Err("unexpected internal argument".into()); }
    let place = shared_translator::placement::resolve_place(&metaflow)?;
    let socket = shared_translator::placement::runtime_path(&place).join(socket_name);
    belief_nexus_translator::text_request_stream(
        &socket, &datom, signal::FrameCapacity::default(), |answer| {
            println!("{answer}");
            true
        },
    )?;
    Ok(())
}
