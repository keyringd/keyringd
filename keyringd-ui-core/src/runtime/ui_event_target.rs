use super::UiEventSender;
use std::rc::Rc;

/// Immutable typed connection owned by the UI thread, without a callback slot.
/// Invokes its handler synchronously; a sender adapter may explicitly enqueue it.
pub struct UiEventTarget<T> {
    deliver: Rc<dyn Fn(T)>,
}
impl<T> Clone for UiEventTarget<T> {
    fn clone(&self) -> Self {
        Self {
            deliver: Rc::clone(&self.deliver),
        }
    }
}
impl<T: 'static> UiEventTarget<T> {
    pub fn new(deliver: impl Fn(T) + 'static) -> Self {
        Self {
            deliver: Rc::new(deliver),
        }
    }
    pub fn send(&self, event: T) {
        (self.deliver)(event);
    }
    pub fn map<U: 'static>(&self, map: impl Fn(U) -> T + 'static) -> UiEventTarget<U> {
        let target = self.clone();
        UiEventTarget::new(move |event| target.send(map(event)))
    }
}
impl<T: Send + 'static> UiEventTarget<T> {
    pub fn from_sender(sender: UiEventSender<T>) -> Self {
        Self::new(move |event| sender.send(event))
    }
}
#[cfg(test)]
mod tests {
    use super::UiEventTarget;
    use std::cell::RefCell;
    use std::rc::Rc;
    keyringd_testkit::scenario! {
        group "UiEventTarget::send" {
            case "Nested notifications are synchronous and do not borrow a slot" {
                verify "invoke a nested notification immediately without holding the callback slot" {
                    let values = Rc::new(RefCell::new(Vec::new()));
                    let output = values.clone();
                    let inner = UiEventTarget::new(move |event| output.borrow_mut().push(event));
                    let output = values.clone();
                    let outer = UiEventTarget::new(move |event| {
                        output.borrow_mut().push(event);
                        inner.send(event + 1);
                    });
                    outer.map(|value| value * 2).send(3);
                    assert_eq!(*values.borrow(), vec![6, 7]);
                }
            }
        }
    }
}
