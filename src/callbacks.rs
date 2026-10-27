#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]

use core::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, MutexGuard, TryLockError};

use crate::writer::{InputPassDescription, SegmentOutput, SegmentReport, SegmentType};
use crate::{binary::borrow_payload, ffi};

pub(crate) struct ReadyCallbackState {
    callback: Mutex<Box<dyn FnMut() + Send + 'static>>,
}

impl ReadyCallbackState {
    pub(crate) fn new(callback: impl FnMut() + Send + 'static) -> Self {
        Self {
            callback: Mutex::new(Box::new(callback)),
        }
    }
}

pub(crate) struct PassDescriptionCallbackState {
    callback: Mutex<Box<dyn FnMut(Option<InputPassDescription>) + Send + 'static>>,
}

impl PassDescriptionCallbackState {
    pub(crate) fn new(callback: impl FnMut(Option<InputPassDescription>) + Send + 'static) -> Self {
        Self {
            callback: Mutex::new(Box::new(callback)),
        }
    }
}

pub(crate) struct SegmentCallbackState {
    callback: Mutex<Box<dyn FnMut(SegmentOutput) + Send + 'static>>,
}

impl SegmentCallbackState {
    pub(crate) fn new(callback: impl FnMut(SegmentOutput) + Send + 'static) -> Self {
        Self {
            callback: Mutex::new(Box::new(callback)),
        }
    }
}

fn catch_callback_panic<F: FnOnce()>(site: &str, f: F) {
    if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(f)) {
        let msg = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>")
            .to_owned();
        forget_nested_panic(std::panic::catch_unwind(AssertUnwindSafe(|| {
            eprintln!("avassetwriter: panic in {site} caught at C ABI boundary: {msg}");
        })));
        forget_nested_panic(std::panic::catch_unwind(AssertUnwindSafe(|| drop(payload))));
    }
}

fn forget_nested_panic(result: std::thread::Result<()>) {
    if let Err(payload) = result {
        std::mem::forget(payload);
    }
}

fn try_lock_callback<'a, T>(site: &str, callback: &'a Mutex<T>) -> Option<MutexGuard<'a, T>> {
    match callback.try_lock() {
        Ok(callback) => Some(callback),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => {
            forget_nested_panic(std::panic::catch_unwind(AssertUnwindSafe(|| {
                eprintln!("avassetwriter: overlapping invocation suppressed in {site}");
            })));
            None
        }
    }
}

pub(crate) unsafe extern "C" fn ready_callback_trampoline(userdata: *mut c_void) {
    if userdata.is_null() {
        return;
    }
    let state = unsafe { &*userdata.cast::<ReadyCallbackState>() };
    let Some(mut callback) = try_lock_callback("ready_callback_trampoline", &state.callback) else {
        return;
    };
    catch_callback_panic("ready_callback_trampoline", &mut *callback);
}

pub(crate) unsafe extern "C" fn ready_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        catch_callback_panic("ready_callback_drop", || {
            drop(unsafe { Box::from_raw(userdata.cast::<ReadyCallbackState>()) });
        });
    }
}

pub(crate) unsafe extern "C" fn pass_description_callback_trampoline(
    payload: *const ffi::AVWOwnedPayload,
    userdata: *mut c_void,
) {
    if userdata.is_null() {
        return;
    }
    let state = unsafe { &*userdata.cast::<PassDescriptionCallbackState>() };
    let payload = borrow_payload::<InputPassDescription>(payload, "pass description").ok();
    let Some(mut callback) =
        try_lock_callback("pass_description_callback_trampoline", &state.callback)
    else {
        return;
    };
    catch_callback_panic("pass_description_callback_trampoline", || {
        callback(payload);
    });
}

pub(crate) unsafe extern "C" fn pass_description_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        catch_callback_panic("pass_description_callback_drop", || {
            drop(unsafe { Box::from_raw(userdata.cast::<PassDescriptionCallbackState>()) });
        });
    }
}

pub(crate) unsafe extern "C" fn segment_callback_trampoline(
    bytes: *const u8,
    byte_len: usize,
    segment_type: i32,
    report_payload: *const ffi::AVWOwnedPayload,
    userdata: *mut c_void,
) {
    if userdata.is_null() {
        return;
    }
    let state = unsafe { &*userdata.cast::<SegmentCallbackState>() };
    let data = if bytes.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(bytes, byte_len) }.to_vec()
    };
    let report = borrow_payload::<SegmentReport>(report_payload, "segment report").ok();
    let Some(mut callback) = try_lock_callback("segment_callback_trampoline", &state.callback)
    else {
        return;
    };
    catch_callback_panic("segment_callback_trampoline", || {
        callback(SegmentOutput {
            data,
            segment_type: SegmentType::from_raw(segment_type),
            report,
        });
    });
}

pub(crate) unsafe extern "C" fn segment_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        catch_callback_panic("segment_callback_drop", || {
            drop(unsafe { Box::from_raw(userdata.cast::<SegmentCallbackState>()) });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    #[test]
    fn ready_callback_invocations_do_not_overlap() {
        let active = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let overlapped = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));

        let state = Box::new(ReadyCallbackState::new({
            let active = Arc::clone(&active);
            let calls = Arc::clone(&calls);
            let overlapped = Arc::clone(&overlapped);
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            move || {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                if active.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlapped.store(true, Ordering::SeqCst);
                }
                if call == 0 {
                    entered.wait();
                    release.wait();
                }
                active.fetch_sub(1, Ordering::SeqCst);
            }
        }));
        let userdata = Box::into_raw(state).cast::<c_void>();
        let address = userdata as usize;
        let thread = std::thread::spawn(move || unsafe {
            ready_callback_trampoline(address as *mut c_void);
        });

        entered.wait();
        unsafe { ready_callback_trampoline(userdata) };
        release.wait();
        thread.join().expect("callback thread");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!overlapped.load(Ordering::SeqCst));
        unsafe { ready_callback_drop(userdata) };
    }

    #[test]
    fn callback_panic_does_not_poison_future_invocations() {
        let calls = Arc::new(AtomicUsize::new(0));
        let state = Box::new(ReadyCallbackState::new({
            let calls = Arc::clone(&calls);
            move || {
                assert!(
                    calls.fetch_add(1, Ordering::SeqCst) != 0,
                    "first invocation"
                );
            }
        }));
        let userdata = Box::into_raw(state).cast::<c_void>();

        unsafe {
            ready_callback_trampoline(userdata);
            ready_callback_trampoline(userdata);
            ready_callback_drop(userdata);
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn panic_payload_drop_cannot_unwind_across_ffi() {
        struct PanicPayload(Arc<AtomicUsize>);

        impl Drop for PanicPayload {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("payload drop");
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let state = Box::new(ReadyCallbackState::new({
            let drops = Arc::clone(&drops);
            move || std::panic::panic_any(PanicPayload(Arc::clone(&drops)))
        }));
        let userdata = Box::into_raw(state).cast::<c_void>();

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            ready_callback_trampoline(userdata);
        }));
        assert!(result.is_ok());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        unsafe { ready_callback_drop(userdata) };
    }

    #[test]
    fn callback_destructor_panic_cannot_unwind_across_ffi() {
        struct PanicOnDrop(Arc<AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("callback drop");
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let state = Box::new(ReadyCallbackState::new({
            let guard = PanicOnDrop(Arc::clone(&drops));
            move || {
                let _ = &guard;
            }
        }));
        let userdata = Box::into_raw(state).cast::<c_void>();

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            ready_callback_drop(userdata);
        }));
        assert!(result.is_ok());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}
