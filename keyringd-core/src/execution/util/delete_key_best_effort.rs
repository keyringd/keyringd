use crate::application::ApplicationError;
use crate::model::device_key::DevicePrivateKeyHandle;
use crate::service::DeviceKeyService;
use crate::util::ErrorChain;

pub(in crate::execution) async fn delete_key_best_effort<D: DeviceKeyService>(
    device_key_service: &D,
    context: &D::OperationContext,
    handle: &DevicePrivateKeyHandle,
) {
    if let Err(error) = device_key_service.delete_key(context, handle).await {
        let error = ApplicationError::from_device_key_error(device_key_service, error);
        tracing::warn!(
            error = %error,
            error_chain = %ErrorChain::new(&error),
            error_debug = ?error,
            "unused device key could not be deleted"
        );
    }
}
