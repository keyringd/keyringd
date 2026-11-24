use super::{ModelCredential, PendingAssertion};
use keyringd_fido::CtapCommand;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Default)]
pub(super) struct State {
    pub(super) credentials: HashMap<Vec<u8>, ModelCredential>,
    pub(super) pending_assertions: VecDeque<PendingAssertion>,
    pub(super) assertion_deadline: Option<Instant>,
    pub(super) sign_count: u32,
    pub(super) next_creation_order: u64,
    pub(super) commands: Vec<CtapCommand>,
}
