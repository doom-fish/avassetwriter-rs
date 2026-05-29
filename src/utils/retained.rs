//! Declarative macro for retain/release wrapper boilerplate.
//!
//! Several `AVFoundation` wrapper types hold a single `Cell<*mut c_void>`
//! pointer to a lazily-materialized, ARC-retained Objective-C object and
//! hand-roll identical `Clone` (rebuild from the JSON payload) and `Drop`
//! (release) implementations. `av_retained!` consolidates that boilerplate
//! into a single audited place.
//!
//! The generated impls preserve the exact behavior of the previous
//! hand-written versions:
//! - `Clone` rebuilds a fresh wrapper from the current payload via
//!   `Self::from_payload(self.payload())` (the previous behavior — these
//!   wrappers do not retain the underlying pointer on clone).
//! - `Drop` swaps the `ptr` cell for null and, when the previous value was
//!   non-null, calls the supplied `release` FFI fn (matching the original
//!   `self.ptr.replace(...)` + `if !ptr.is_null()` guards).
//!
//! Wrappers whose teardown carries extra logic beyond release + null-check
//! (e.g. `MetadataItemValueRequest`, which holds a bare `*mut c_void` and
//! nulls the field in place) are intentionally left hand-written.

/// Generate `Clone` and/or `Drop` impls for a `Cell`-based retain/release
/// pointer wrapper.
///
/// Variants:
/// - `Clone` + `Drop`:
///   `av_retained!(Ty, release = path::release);`
/// - `Drop` only:
///   `av_retained!(Ty, drop_only, release = path::release);`
macro_rules! av_retained {
    // Cell-based wrapper: Clone (rebuild from payload) + Drop (release)
    ($ty:ty, release = $release:path $(,)?) => {
        impl Clone for $ty {
            fn clone(&self) -> Self {
                Self::from_payload(self.payload())
            }
        }

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

    // Cell-based wrapper: Drop only (release)
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
