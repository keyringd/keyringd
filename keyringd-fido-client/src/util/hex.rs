pub(crate) fn encode(value: &[u8]) -> String {
    use std::fmt::Write;

    let mut output = String::with_capacity(value.len() * 2);
    for byte in value {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
