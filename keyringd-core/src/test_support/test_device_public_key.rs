use p256::elliptic_curve::sec1::ToSec1Point;

use keyringd_core::model::device_key::{DeviceKeyAlgorithm, DevicePublicKey};

pub(crate) fn test_device_public_key(algorithm: DeviceKeyAlgorithm) -> DevicePublicKey {
    let point = p256::SecretKey::from_slice(&[1; 32])
        .expect("fixed test key is valid")
        .public_key()
        .to_sec1_point(false);
    assert!(matches!(algorithm, DeviceKeyAlgorithm::EcdsaP256));
    DevicePublicKey::from_ecdsa_p256(point.as_bytes()).unwrap()
}
