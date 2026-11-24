use super::TrustEntryKind;
pub struct TrustEntryViewProps {
    pub kind: TrustEntryKind,
    pub description: &'static str,
    pub value: String,
}
