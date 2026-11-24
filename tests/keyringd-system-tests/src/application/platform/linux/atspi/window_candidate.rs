pub(super) struct WindowCandidate {
    pub(super) bounds: (i32, i32, i32, i32),
    pub(super) active: bool,
    pub(super) modal: bool,
    pub(super) dialog: bool,
}
