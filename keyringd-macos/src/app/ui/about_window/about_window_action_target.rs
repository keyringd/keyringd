use super::AboutWindow;
use keyringd_ui_core::text as ui_text;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSObject;

use std::rc::Weak;

pub(super) struct AboutWindowActionTargetIvars {
    owner: Weak<AboutWindow>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeyringdAboutWindowActionTarget"]
    #[ivars = AboutWindowActionTargetIvars]
    pub(super) struct AboutWindowActionTarget;

    impl AboutWindowActionTarget {
        #[unsafe(method(showAbout:))]
        fn show_about(&self, _sender: Option<&AnyObject>) {
            self.with_owner(AboutWindow::show);
        }

        #[unsafe(method(closeAbout:))]
        fn close_about(&self, _sender: Option<&AnyObject>) {
            self.with_owner(AboutWindow::close);
        }

        #[unsafe(method(openWebsite:))]
        fn open_website(&self, _sender: Option<&AnyObject>) {
            self.with_owner(|owner| owner.open_url(ui_text::ABOUT_WEBSITE_URL));
        }

        #[unsafe(method(reportIssue:))]
        fn report_issue(&self, _sender: Option<&AnyObject>) {
            self.with_owner(|owner| owner.open_url(ui_text::ABOUT_ISSUES_URL));
        }

        #[unsafe(method(showLicense:))]
        fn show_license(&self, _sender: Option<&AnyObject>) {
            self.with_owner(AboutWindow::show_license);
        }

        #[unsafe(method(showThirdPartyLicenses:))]
        fn show_third_party_licenses(&self, _sender: Option<&AnyObject>) {
            self.with_owner(AboutWindow::show_third_party_licenses);
        }

    }
);

impl AboutWindowActionTarget {
    pub(super) fn new(mtm: MainThreadMarker, owner: Weak<AboutWindow>) -> Retained<Self> {
        let allocated = mtm
            .alloc()
            .set_ivars(AboutWindowActionTargetIvars { owner });
        unsafe { msg_send![super(allocated), init] }
    }

    fn with_owner(&self, action: impl FnOnce(&AboutWindow)) {
        if let Some(owner) = self.ivars().owner.upgrade() {
            action(&owner);
        }
    }
}
