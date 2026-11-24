pub enum PinPromptEvent {
    PinChanged(String),
    ConfirmationChanged(String),
    SubmitRequested,
    CancelRequested,
    Dismissed,
}
