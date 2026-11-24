use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSBox, NSBoxType, NSColor, NSFont, NSLayoutAttribute, NSLayoutConstraintOrientation,
    NSLayoutPriorityDefaultLow, NSStackView, NSStackViewDistribution, NSTextField, NSTitlePosition,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSSize, NSString};

pub(crate) struct Layout;

impl Layout {
    pub(crate) fn label(
        text: &str,
        size: f64,
        bold: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSTextField> {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        let font = if bold {
            NSFont::boldSystemFontOfSize(size)
        } else {
            NSFont::systemFontOfSize(size)
        };
        label.setFont(Some(&font));
        if !bold {
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        label
    }

    pub(crate) fn stack(
        views: &[&NSView],
        orientation: NSUserInterfaceLayoutOrientation,
        spacing: f64,
        mtm: MainThreadMarker,
    ) -> Retained<NSStackView> {
        let stack = NSStackView::stackViewWithViews(&NSArray::from_slice(views), mtm);
        stack.setOrientation(orientation);
        stack.setDistribution(NSStackViewDistribution::Fill);
        stack.setAlignment(
            if orientation == NSUserInterfaceLayoutOrientation::Vertical {
                NSLayoutAttribute::Leading
            } else {
                NSLayoutAttribute::CenterY
            },
        );
        stack.setSpacing(spacing);
        if orientation == NSUserInterfaceLayoutOrientation::Vertical {
            for view in views {
                Self::fill_width(view, &stack);
            }
        }
        stack
    }

    pub(crate) fn fill_width(view: &NSView, container: &NSView) {
        Self::prepare(view);
        view.widthAnchor()
            .constraintEqualToAnchor(&container.widthAnchor())
            .setActive(true);
    }

    pub(crate) fn flexible_width(view: &NSView) {
        view.setContentHuggingPriority_forOrientation(
            NSLayoutPriorityDefaultLow - 1.0,
            NSLayoutConstraintOrientation::Horizontal,
        );
        view.setContentCompressionResistancePriority_forOrientation(
            NSLayoutPriorityDefaultLow,
            NSLayoutConstraintOrientation::Horizontal,
        );
    }

    pub(crate) fn actions(buttons: &[&NSView], mtm: MainThreadMarker) -> Retained<NSStackView> {
        let spacer = NSView::new(mtm);
        let mut views = vec![&*spacer];
        views.extend_from_slice(buttons);
        Self::stack(
            &views,
            NSUserInterfaceLayoutOrientation::Horizontal,
            8.0,
            mtm,
        )
    }

    pub(crate) fn group(
        views: &[&NSView],
        warning: bool,
        mtm: MainThreadMarker,
    ) -> Retained<NSView> {
        let content = NSView::new(mtm);
        let background = NSBox::new(mtm);
        background.setBoxType(if warning {
            NSBoxType::Custom
        } else {
            NSBoxType::Primary
        });
        background.setTitlePosition(NSTitlePosition::NoTitle);
        background.setContentViewMargins(NSSize::new(0.0, 0.0));
        if warning {
            background.setFillColor(&NSColor::controlBackgroundColor());
            background.setBorderColor(&NSColor::systemOrangeColor());
            background.setCornerRadius(8.0);
        }
        content.addSubview(&background);
        Self::pin(&background, &content, 0.0, 0.0, 0.0, 0.0);
        let stack = Self::stack(views, NSUserInterfaceLayoutOrientation::Vertical, 12.0, mtm);
        content.addSubview(&stack);
        Self::pin(&stack, &content, 16.0, 16.0, 16.0, 16.0);
        content
    }

    /// Keep the chosen reading width, but derive the window height from content.
    pub(crate) fn fit_window(content: &NSView) {
        content.layoutSubtreeIfNeeded();
        if let Some(window) = content.window() {
            window.setContentSize(content.fittingSize());
        }
    }

    pub(crate) fn pin_top(view: &NSView, container: &NSView) {
        Self::prepare(view);
        view.topAnchor()
            .constraintEqualToAnchor(&container.topAnchor())
            .setActive(true);
        view.leadingAnchor()
            .constraintEqualToAnchor(&container.leadingAnchor())
            .setActive(true);
        view.trailingAnchor()
            .constraintEqualToAnchor(&container.trailingAnchor())
            .setActive(true);
    }

    /// A table creates its row before attaching it to the window. Defer focus
    /// until AppKit has installed the view and its field editor can accept it.
    pub(crate) fn focus_after_attachment(view: &NSView, mtm: objc2::MainThreadMarker) {
        use objc2::Message;
        let view = dispatch2::MainThreadBound::new(view.retain(), mtm);
        dispatch2::DispatchQueue::main().exec_async(move || {
            let mtm = objc2::MainThreadMarker::new().expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
            let view = view.get(mtm);
            if let Some(window) = view.window() {
                window.makeFirstResponder(Some(view));
            }
        });
    }

    pub(crate) fn prepare(view: &NSView) {
        view.setTranslatesAutoresizingMaskIntoConstraints(false);
    }

    pub(crate) fn pin(
        view: &NSView,
        container: &NSView,
        top: f64,
        leading: f64,
        bottom: f64,
        trailing: f64,
    ) {
        Self::prepare(view);
        view.topAnchor()
            .constraintEqualToAnchor_constant(&container.topAnchor(), top)
            .setActive(true);
        view.leadingAnchor()
            .constraintEqualToAnchor_constant(&container.leadingAnchor(), leading)
            .setActive(true);
        view.bottomAnchor()
            .constraintEqualToAnchor_constant(&container.bottomAnchor(), -bottom)
            .setActive(true);
        view.trailingAnchor()
            .constraintEqualToAnchor_constant(&container.trailingAnchor(), -trailing)
            .setActive(true);
    }

    pub(crate) fn height(view: &NSView, height: f64) {
        view.heightAnchor()
            .constraintEqualToConstant(height)
            .setActive(true);
    }

    pub(crate) fn center_x(view: &NSView, container: &NSView) {
        view.centerXAnchor()
            .constraintEqualToAnchor(&container.centerXAnchor())
            .setActive(true);
    }

    pub(crate) fn center_y(view: &NSView, container: &NSView, offset: f64) {
        view.centerYAnchor()
            .constraintEqualToAnchor_constant(&container.centerYAnchor(), offset)
            .setActive(true);
    }

    pub(crate) fn width(view: &NSView, width: f64) {
        view.widthAnchor()
            .constraintEqualToConstant(width)
            .setActive(true);
    }
}
