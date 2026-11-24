use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSImage, NSToolbar, NSToolbarDelegate, NSToolbarFlexibleSpaceItemIdentifier, NSToolbarItem,
    NSToolbarItemIdentifier,
};
use objc2_foundation::{NSArray, NSObject, NSString};

use keyringd_ui_core::runtime::UiEventTarget;

use super::ProfileListEvent;

const ADD_PROFILE_IDENTIFIER: &str = "com.keyringd.toolbar.add-profile";

pub(crate) struct ProfileListToolbarDelegateIvars {
    events: UiEventTarget<ProfileListEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdProfileListToolbarDelegate"]
    #[ivars = ProfileListToolbarDelegateIvars]
    pub(crate) struct ProfileListToolbarDelegate;

    unsafe impl NSObjectProtocol for ProfileListToolbarDelegate {}

    impl ProfileListToolbarDelegate {
        #[unsafe(method(addProfile:))]
        fn add_profile(&self, _sender: Option<&AnyObject>) {
            self.ivars().events.send(ProfileListEvent::AddRequested);
        }
    }

    unsafe impl NSToolbarDelegate for ProfileListToolbarDelegate {
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn toolbar_item_for_identifier(
            &self,
            _toolbar: &NSToolbar,
            identifier: &NSToolbarItemIdentifier,
            _inserted: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            if identifier == &*NSString::from_str(ADD_PROFILE_IDENTIFIER) {
                let mtm = MainThreadMarker::new().expect("the toolbar delegate should run on the AppKit thread because AppKit invokes its UI callbacks there");
                let item =
                    NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), identifier);
                item.setAutovalidates(false);
                item.setLabel(&NSString::from_str(ui_text::PROFILE_LIST_ADD));
                item.setPaletteLabel(&NSString::from_str(ui_text::PROFILE_LIST_ADD));
                item.setToolTip(Some(&NSString::from_str(ui_text::PROFILE_LIST_ADD_TOOLTIP)));
                if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &NSString::from_str("plus"),
                    Some(&NSString::from_str(ui_text::PROFILE_LIST_ADD)),
                ) {
                    item.setImage(Some(&image));
                }
                unsafe {
                    item.setTarget(Some(self));
                    item.setAction(Some(sel!(addProfile:)));
                }
                Some(item)
            } else {
                None
            }
        }

        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn toolbar_default_item_identifiers(
            &self,
            _toolbar: &NSToolbar,
        ) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            let add = NSString::from_str(ADD_PROFILE_IDENTIFIER);
            unsafe { NSArray::from_slice(&[NSToolbarFlexibleSpaceItemIdentifier, &add]) }
        }

        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn toolbar_allowed_item_identifiers(
            &self,
            _toolbar: &NSToolbar,
        ) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            let add = NSString::from_str(ADD_PROFILE_IDENTIFIER);
            unsafe { NSArray::from_slice(&[NSToolbarFlexibleSpaceItemIdentifier, &add]) }
        }
    }
);

impl ProfileListToolbarDelegate {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        events: UiEventTarget<ProfileListEvent>,
    ) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(ProfileListToolbarDelegateIvars { events });
        unsafe { msg_send![super(allocated), init] }
    }
}
