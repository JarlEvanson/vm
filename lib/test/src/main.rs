use logbuffer::allocated::AllocatedLogBuffer;

fn main() {
    let logbuffer = AllocatedLogBuffer::new(512 * 1024, 1024, "Hello");

    let mut sequence = logbuffer.tail_sequence();
    let mut buffer = [0; 4096];

    let mut processed_messages = 0;
    while let Some(message) = logbuffer.read(sequence, &mut buffer) {
        println!(
            "item {processed_messages}: retrieved sequence {}: {:#?}",
            message.sequence, message.buffer
        );

        sequence = message.sequence + 1;
        processed_messages += 1;
        assert_eq!(processed_messages, 1);
    }

    assert_eq!(processed_messages, 1);
}
