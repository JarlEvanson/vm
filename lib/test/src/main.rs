use std::sync::atomic::Ordering;

use logbuffer::allocated::AllocatedLogBuffer;

fn main() {
    let logbuffer = AllocatedLogBuffer::new(512 * 1024, 1024, "Hello");

    let mut sequence = logbuffer.tail_sequence();
    let mut buffer = [0; 4096];

    let message = logbuffer.read(sequence, &mut buffer).unwrap();
    sequence = message.sequence;
    assert!(logbuffer.read(message.sequence + 1, &mut buffer).is_none());

    let message_contents = " World!";
    let message = logbuffer
        .reserve(message_contents.len())
        .expect("failed to reserve message space");

    for (i, &byte) in message_contents.as_bytes().iter().enumerate() {
        message.buffer()[i].store(byte, Ordering::Relaxed);
    }

    message.finalize();

    let message = logbuffer.read(sequence, &mut buffer).unwrap();
    assert!(logbuffer.read(message.sequence + 1, &mut buffer).is_none());
}
