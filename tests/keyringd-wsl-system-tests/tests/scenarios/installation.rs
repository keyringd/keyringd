use keyringd_testkit::scenario;
use keyringd_wsl_system_tests::WslPackageInstallation;

scenario! {
    case "Starts the Windows companion from the proxy with only the WSL package installed" {
        given "WSL packages installed in dedicated distribution" {
            let installation = WslPackageInstallation::from_environment().unwrap();
        }
        when "Check installed proxy and Socket settings" {
            let output = installation.verify().unwrap();
        }
        then "Published Socket and Service is enabled and Proxy starts using the included Companion as per the package settings" {
            keyringd_testkit::evidence_text!("Installed WSL package", output);
        }
    }
}
