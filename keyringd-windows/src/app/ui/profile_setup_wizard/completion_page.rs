use windows_reactor::{AccessibilityExt, Element, LayoutExt, ThemeRef, border, text_block};

use windows_reactor::{InfoBar, InfoBarSeverity, TextStyleExt, vstack};

pub(crate) struct CompletionPage;

impl CompletionPage {
    pub(crate) fn render(description: String, notice: Option<String>) -> Element {
        let notice: Element = notice
            .map(|message| {
                InfoBar::new(message)
                    .severity(InfoBarSeverity::Error)
                    .is_open(true)
                    .into()
            })
            .unwrap_or_else(|| border(text_block("")).height(0.0).into());
        vstack((
            text_block(description)
                .automation_id("profile-setup-completed-description")
                .wrap()
                .foreground(ThemeRef::SecondaryText),
            notice,
        ))
        .spacing(12.0)
        .into()
    }
}
