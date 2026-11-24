use super::InteractionId;

/// Set the response value to the dialog and the ID of the corresponding request. Prevent misuse of responses to other requests.
pub struct InteractionResponse<T> {
    id: InteractionId,
    value: T,
}

impl<T> InteractionResponse<T> {
    pub fn new(id: InteractionId, value: T) -> Self {
        Self { id, value }
    }
    pub const fn id(&self) -> InteractionId {
        self.id
    }
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn into_value(self) -> T {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OperationId;

    keyringd_testkit::scenario! {
        group "InteractionResponse" {
            case "A response can be inspected before its value is consumed without losing its request ID" {
                verify "borrowing preserves the response for request matching and ownership transfer" {
                    let id = InteractionId::new(OperationId::new(7), 3);
                    let response = InteractionResponse::new(id, String::from("approved"));
                    assert_eq!(response.value(), "approved");
                    assert_eq!(response.id(), id);
                    assert_eq!(response.into_value(), "approved");
                }
            }
        }
    }
}
