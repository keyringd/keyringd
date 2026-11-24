use keyringd_fido::CtapCommand;

pub(crate) fn trace_bytes(
    boundary: &'static str,
    direction: &'static str,
    value: &[u8],
    command: Option<CtapCommand>,
) {
    tracing::trace!(
        target: "keyringd_fido_client::wire",
        boundary,
        direction,
        command = ?command,
        length = value.len(),
        wire = %super::hex::encode(value),
        "FIDO wire data"
    );
}

pub(crate) fn trace_ctap_response(response: &keyringd_fido::CtapResponse, command: CtapCommand) {
    tracing::trace!(
        target: "keyringd_fido_client::wire",
        boundary = "ctap",
        direction = "in",
        command = ?Some(command),
        length = response.payload().len() + 1,
        wire = %super::hex::encode(&response.to_bytes()),
        "FIDO wire data"
    );
}
