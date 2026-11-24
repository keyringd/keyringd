/// Environmental reasons why Hybrid is not available, such as the absence or disabling of the Bluetooth adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HybridTransportUnavailableReason {
    NoBluetoothAdapter,
    BluetoothDisabled,
}
