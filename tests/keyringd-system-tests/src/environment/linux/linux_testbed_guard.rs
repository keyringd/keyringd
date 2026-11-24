use super::{
    Environment, HiddenBluetoothAdapters, PoweredOffVirtualBluetooth, RemoteHybridAdvertisements,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::runtime::Runtime;
pub struct LinuxTestbedGuard {
    environment: Environment,
    runtime: Arc<Runtime>,
    state: TestbedOverride,
}
enum TestbedOverride {
    Disabled(PoweredOffVirtualBluetooth),
    Hidden(HiddenBluetoothAdapters),
    Remote(RemoteHybridAdvertisements),
}
impl LinuxTestbedGuard {
    pub(crate) fn disabled(environment: Environment, runtime: Arc<Runtime>) -> Result<Self> {
        let state =
            TestbedOverride::Disabled(runtime.block_on(environment.power_off_virtual_bluetooth())?);
        Ok(Self {
            environment,
            runtime,
            state,
        })
    }
    pub(crate) fn hidden(environment: Environment, runtime: Arc<Runtime>) -> Result<Self> {
        let state =
            TestbedOverride::Hidden(runtime.block_on(environment.without_bluetooth_adapters())?);
        Ok(Self {
            environment,
            runtime,
            state,
        })
    }
    pub(crate) fn remote(environment: Environment, runtime: Arc<Runtime>) -> Result<Self> {
        let state = TestbedOverride::Remote(
            runtime.block_on(environment.use_remote_hybrid_advertisements())?,
        );
        Ok(Self {
            environment,
            runtime,
            state,
        })
    }
    pub fn restore(self) -> Result<()> {
        match self.state {
            TestbedOverride::Disabled(guard) => {
                self.runtime.block_on(guard.restore(&self.environment))
            }
            TestbedOverride::Hidden(guard) => {
                self.runtime.block_on(guard.restore(&self.environment))
            }
            TestbedOverride::Remote(guard) => {
                self.runtime.block_on(guard.restore(&self.environment))
            }
        }
    }
}
