use crate::scenario_steps;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(target_os = "linux")]
use keyringd_testkit::evidence_log;
use keyringd_testkit::{evidence_image, evidence_text, scenario};

scenario! {
    case "Creates a Passkey through the PXP BLE advertisement relay" {
        skip_if: !keyringd_system_tests::authenticator::Authenticators::hybrid_available() => "Requires a virtual phone and a Hybrid test tunnel";
        given "Product creation wizard using virtual phones and empty destinations" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment
                .authenticators()
                .create_hybrid_phone(Default::default())
                .unwrap();
            #[cfg(target_os = "linux")]
            let _advertisements = environment
                .testbed()
                .remote_hybrid_advertisements()
                .unwrap();
            let configuration = ApplicationConfiguration {
                #[cfg(any(target_os = "macos", windows))]
                accept_remote_hybrid_advertisements: true,
                ..Default::default()
            };
            environment.application().start(configuration).unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
        }
        when "Create Passkey with separate QRs, authorize Device Key and return to the list" {
            let advertisements = scenario_steps::profile_setup::hybrid_profiles::create(
                &mut environment,
                &phone,
                "pxp-created",
            )
            .unwrap();
        }
        then "Real Hybrid communication is successful and only one profile is saved" {
            assert_eq!(advertisements[0].tunnel_domain, "cable.ua5v.com");
            let stored = environment.application().stored_profiles().unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0]["displayName"], "pxp-created");
            environment
                .application()
                .capture_screenshot("Profile setup result")
                .unwrap();
            evidence_text!("Create Passkey tunnel", &advertisements[0].tunnel_path);
            evidence_text!(
                "Authorize Device Key tunnel",
                &advertisements[1].tunnel_path
            );
        }
    }
}

#[cfg(target_os = "linux")]
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver, ProfileSetupStep,
};

scenario! {
    #[cfg(target_os = "linux")]
    case "Creates a Passkey through btvirt and BlueZ" {
        skip_if: std::env::var_os("KEYRINGD_E2E_BLE_ADVERTISER_ADDRESS").is_none()
            => "requires btvirt controllers on a host with hci_vhci";
        given "Prepare a BtVirt phone side, a PC side controller, and a virtual phone" {
            let mut environment = NativeSystemTestEnvironment::new().unwrap();
            let phone = environment
                .authenticators()
                .create_hybrid_phone(Default::default())
                .unwrap();
            let advertiser = environment.testbed().ble_advertiser().unwrap();
            environment
                .application()
                .start(ApplicationConfiguration::default())
                .unwrap();
            environment
                .application()
                .profile_setup_window()
                .open()
                .unwrap();
        }
        when "Start creating a new Passkey" {
            scenario_steps::profile_setup::form::submit_creation(
                &mut environment,
                "hybrid-real-ble-profile",
            )
            .unwrap();
        }
        then "Shows how to create a new Passkey" {
            scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                .unwrap();
        }
        when "Read the QR on the screen and send FFF9 ads for each operation as a virtual BLE" {
            let (first_qr, first_screenshot) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(None)
                .unwrap();
            evidence_image!("Create Passkey QR over BLE", first_screenshot);
            let first_ad = phone.scan_hybrid_qr(&first_qr).unwrap();
            advertiser.advertise(&first_ad).unwrap();
            evidence_text!("First Hybrid tunnel", &first_ad.tunnel_path);
        }
        then "Waiting for approval for the new Device Key is displayed" {
            scenario_steps::profile_setup::progress::wait_for_step(
                &mut environment,
                ProfileSetupStep::AuthorizeNewDevice,
            )
            .unwrap();
        }
        when "Read the QR on the screen and send FFF9 ads for each operation as a virtual BLE" {
            let (second_qr, second_screenshot) = environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .qr_view()
                .unwrap()
                .read_hybrid_qr(Some(&first_qr))
                .unwrap();
        }
        then "The QR for each creation operation is different from the previous one." {
            assert_ne!(first_qr, second_qr, "each operation needs a fresh QR");
            evidence_image!("Authorize Device Key QR over BLE", second_screenshot);
        }
        when "Read the QR on the screen and send FFF9 ads for each operation as a virtual BLE" {
            let second_ad = phone.scan_hybrid_qr(&second_qr).unwrap();
            advertiser.advertise(&second_ad).unwrap();
            evidence_text!("Second Hybrid tunnel", &second_ad.tunnel_path);
        }
        then "Created with Ads from Bluez/BtlePlug" {
            scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
            environment
                .application()
                .capture_screenshot("btvirt Hybrid profile created")
                .unwrap();
            let journal = environment.application().agent_journal().unwrap();
            evidence_log!("BlueZ/btleplug Hybrid discovery", &journal);
        }
    }
}
