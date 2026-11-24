//! SSH wire-format codecs and errors produced at byte boundaries.
//!
//! Public `from_bytes`/`to_bytes` operations return a codec error named for the
//! encoded type. Errors from value construction stay with the value type, and
//! errors from operations such as certificate issuance stay with the operation.

mod ssh_agent_codec_error;
mod ssh_reader;
mod ssh_writer;

pub use ssh_agent_codec_error::SshAgentCodecError;
pub use ssh_reader::SshReader;
pub use ssh_writer::SshWriter;
