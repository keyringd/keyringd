/// Keep confirmation and result dialogs at a consistent starting width.
pub(crate) fn message_dialog_builder() -> libadwaita::builders::MessageDialogBuilder {
    libadwaita::MessageDialog::builder().default_width(380)
}
