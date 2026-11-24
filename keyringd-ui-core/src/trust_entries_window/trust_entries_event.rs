use super::{TrustEntriesState, TrustEntryKind};

#[derive(Clone)]
pub enum TrustEntriesEvent {
    CopyRequested(TrustEntryKind),
    EntriesChanged(TrustEntriesState),
    ProfileRemoved,
    CloseRequested,
}
