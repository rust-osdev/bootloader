use core::cell::UnsafeCell;

pub struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    pub const fn new(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }

    /// Gets a mutable pointer to the wrapped value.
    ///
    /// ## Safety
    /// Ensure that the access is unique (no active references, mutable or not).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

unsafe impl<T> Send for RacyCell<T> where T: Send {}
unsafe impl<T: Sync> Sync for RacyCell<T> {}
