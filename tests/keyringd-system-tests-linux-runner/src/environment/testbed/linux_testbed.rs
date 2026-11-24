use super::{TestbedExecutionRequest, hybrid_port_forward, image};

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, ensure};

use super::system_container::SystemContainer;
use super::virtual_authenticator::UsbIpBroker;
use super::virtual_bluetooth::VirtualBluetooth;

pub(crate) struct LinuxTestbed;

impl LinuxTestbed {
    pub(crate) fn execute(request: TestbedExecutionRequest) -> Result<()> {
        ensure_root()?;
        let interrupted = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&interrupted);
        ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))
            .context("failed to install the interrupt handler")?;
        Self::run(&request, &interrupted)
    }

    fn run(request: &TestbedExecutionRequest, interrupted: &AtomicBool) -> Result<()> {
        image::prepare(request)?;

        let bluetooth = VirtualBluetooth::start(&request.runtime_directory())?;

        let _hybrid_port_forward = hybrid_port_forward::HybridPortForward::start()?;

        let mut usb = UsbIpBroker::start(request)?;
        let machine = SystemContainer::boot(request, usb.socket(), bluetooth.as_ref(), interrupted);
        let mut machine = match machine {
            Ok(machine) => machine,
            Err(error) => {
                return crate::util::finish([
                    ("system container startup", Err(error)),
                    ("USB/IP authenticator detach", usb.stop()),
                ]);
            }
        };
        let tests = machine.execute_tests(interrupted, bluetooth.as_ref());
        if tests.is_err() {
            machine.print_diagnostics();
        }
        let detach = usb.stop();
        let shutdown = machine.shutdown();
        crate::util::finish([
            ("E2E tests", tests),
            ("system container shutdown", shutdown),
            ("USB/IP authenticator detach", detach),
        ])
    }
}

fn ensure_root() -> Result<()> {
    let output = crate::util::command::output("id", ["-u"], Path::new("/"))?;
    ensure!(
        output.trim() == "0",
        "Linux system testbed execution must run as root"
    );
    Ok(())
}
