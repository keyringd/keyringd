use keyringd_ui_core::runtime::UiEventTarget;
use windows_reactor::{
    AccessibilityExt, BackgroundExt, Element, GridChildExt, GridLength, Icon, LayoutExt,
    PaddingExt, ScrollBarVisibility, TextStyleExt, ThemeRef, Thickness, TooltipExt, border, button,
    grid, scroll_viewer, text_block, vstack,
};

use keyringd_ui_core::trust_entries_window::TrustEntriesEvent;

use keyringd_ui_core::trust_entries_window::TrustEntryViewProps;

pub(crate) struct TrustEntryView;

impl TrustEntryView {
    pub(crate) fn render(
        props: TrustEntryViewProps,
        on_event: UiEventTarget<TrustEntriesEvent>,
    ) -> Element {
        let title = props.kind.title();
        let button_label = props.kind.copy_label();
        let event = on_event;
        let kind = props.kind;
        let copy = button("")
            .icon(Icon::font("\u{E8C8}"))
            .tooltip(button_label)
            .automation_name(button_label)
            .on_click(move || event.send(TrustEntriesEvent::CopyRequested(kind)));
        let value = border(
            scroll_viewer(
                text_block(props.value)
                    .selectable()
                    .automation_id(format!(
                        "trust-entry-value-{}",
                        button_label.trim_start_matches("Copy ")
                    ))
                    .font_family("Consolas")
                    .automation_name(title),
            )
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden)
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled),
        )
        .height(36.0)
        .padding(Thickness::xy(10.0, 7.0))
        .background(ThemeRef::ControlFill)
        .border_brush(ThemeRef::ControlStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(4.0);
        vstack((
            text_block(title).semibold(),
            text_block(props.description)
                .wrap()
                .foreground(ThemeRef::SecondaryText),
            grid((value.grid_column(0), copy.grid_column(1)))
                .columns([GridLength::STAR, GridLength::Auto])
                .column_spacing(8.0),
        ))
        .spacing(5.0)
        .margin(Thickness::xy(12.0, 4.0))
        .into()
    }
}
