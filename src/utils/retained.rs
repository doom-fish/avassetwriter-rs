//! Declarative macro for `Cell`-based ARC release boilerplate.

/// Generate `Drop` for a `Cell`-based retained pointer wrapper.
macro_rules! av_retained {
    ($ty:ty, drop_only, release = $release:path $(,)?) => {
        impl Drop for $ty {
            fn drop(&mut self) {
                let ptr = self.ptr.replace(core::ptr::null_mut());
                if !ptr.is_null() {
                    unsafe {
                        $release(ptr);
                    }
                }
            }
        }
    };
}

pub(crate) use av_retained;
