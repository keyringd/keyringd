#[derive(Debug, Eq, PartialEq)]
pub enum TrustEntriesCommand {
    Copy {
        value: String,
        success_message: &'static str,
    },
    Close,
}
