use crate::system::application_resources::{APPLICATION_LICENSE, THIRD_PARTY_LICENSES};

use keyringd_ui_core::text as ui_text;
use windows_reactor::{
    AccessibilityExt, GridChildExt, GridLength, HorizontalAlignment, LayoutExt, PaddingExt,
    ScrollBarVisibility, button, grid, scroll_viewer, text_block,
};

use crate::app::ui::AuxiliaryWindow;
use keyringd_ui_core::about_window::LicenseDocument;

pub(crate) struct ThirdPartyLicensesWindow;
impl ThirdPartyLicensesWindow {
    pub(crate) fn open(owner: usize, document: LicenseDocument) -> anyhow::Result<()> {
        AuxiliaryWindow::open_modal(document.title(), (760.0, 620.0), owner, move |_, window| {
            let contents = match document {
                LicenseDocument::Application => APPLICATION_LICENSE,
                LicenseDocument::ThirdParty => THIRD_PARTY_LICENSES,
            };
            let text = text_block(contents)
                .wrap()
                .selectable()
                .automation_id("license-text")
                .automation_name(ui_text::ABOUT_LICENSE_ACCESSIBILITY)
                .padding(24.0);
            let scroll = scroll_viewer(text)
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
            let close = window.clone();
            let footer = button(ui_text::CLOSE)
                .on_click(move || close.close())
                .horizontal_alignment(HorizontalAlignment::Right)
                .margin(16.0)
                .grid_row(1);
            window.content(
                grid((scroll, footer))
                    .rows([GridLength::STAR, GridLength::Auto])
                    .into(),
                true,
            )
        })
    }
}
