#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestFidoMode {
    Success,
    Hold,
    Failure,
    HybridHold,
    BluetoothDisabled,
    NoBluetoothAdapter,
    NoCredentials,
}
