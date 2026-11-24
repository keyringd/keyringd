use std::marker::PhantomData;
use std::rc::Rc;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

pub(crate) struct ComApartment(PhantomData<Rc<()>>);
impl ComApartment {
    pub(crate) fn new() -> windows::core::Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self(PhantomData))
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}
