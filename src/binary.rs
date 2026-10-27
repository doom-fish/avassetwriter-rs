use std::any::Any;
use std::mem::{align_of, size_of};
use std::ptr;

use crate::error::AVWriterError;
use crate::ffi::{self, AVWArray, AVWOwnedPayload, AVWString};

#[derive(Default)]
pub struct Arena {
    allocations: Vec<Box<dyn Any>>,
}

impl Arena {
    pub fn store<T: 'static>(&mut self, value: T) -> *const T {
        let value = Box::new(value);
        let pointer = ptr::from_ref(value.as_ref());
        self.allocations.push(value);
        pointer
    }

    pub fn array<T: 'static>(&mut self, values: Vec<T>) -> AVWArray {
        let raw = AVWArray {
            data: values.as_ptr().cast(),
            length: values.len(),
        };
        self.allocations.push(Box::new(values));
        raw
    }
}

pub trait ToAbi {
    type Abi: Copy + 'static;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi;
}

pub trait FromAbi: ToAbi + Sized {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError>;
}

pub struct Argument<T> {
    raw: T,
    _arena: Arena,
}

impl<T> Argument<T> {
    pub const fn as_ptr(&self) -> *const T {
        ptr::from_ref(&self.raw)
    }
}

pub fn payload_arg<T: ToAbi + ?Sized>(value: &T) -> Argument<T::Abi> {
    let mut arena = Arena::default();
    let raw = value.to_abi(&mut arena);
    Argument { raw, _arena: arena }
}

pub fn string_arg(value: &str, context: &str) -> Result<Argument<AVWString>, AVWriterError> {
    if value.as_bytes().contains(&0) {
        return Err(AVWriterError::InvalidArgument(format!(
            "{context} contained NUL byte"
        )));
    }
    Ok(payload_arg(value))
}

pub struct OwnedPayload(pub *mut AVWOwnedPayload);

impl Drop for OwnedPayload {
    fn drop(&mut self) {
        unsafe { ffi::avw_payload_free(self.0) };
    }
}

pub fn borrow_payload<T: FromAbi>(
    pointer: *const AVWOwnedPayload,
    context: &str,
) -> Result<T, AVWriterError> {
    let envelope = unsafe { pointer.as_ref() }.ok_or_else(|| {
        AVWriterError::InvalidState(format!("swift bridge returned no {context} payload"))
    })?;
    let raw = unsafe { envelope.value.cast::<T::Abi>().as_ref() }.ok_or_else(|| {
        AVWriterError::InvalidState(format!("swift bridge returned no {context} value"))
    })?;
    T::from_abi(raw)
}

pub fn take_payload<T: FromAbi>(
    pointer: *mut AVWOwnedPayload,
    context: &str,
) -> Result<T, AVWriterError> {
    let owned = OwnedPayload(pointer);
    borrow_payload(owned.0, context)
}

pub fn take_swift_string(pointer: *mut AVWOwnedPayload) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    take_payload(pointer, "string").ok()
}

pub fn array_values<T>(raw: &AVWArray) -> Result<&[T], AVWriterError> {
    if raw.length == 0 {
        return Ok(&[]);
    }
    if raw.data.is_null()
        || (raw.data as usize) % align_of::<T>() != 0
        || raw.length > isize::MAX as usize / size_of::<T>().max(1)
    {
        return Err(AVWriterError::InvalidState(
            "swift bridge returned an invalid array".into(),
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(raw.data.cast::<T>(), raw.length) })
}

impl ToAbi for str {
    type Abi = AVWString;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        let bytes = arena.array(self.as_bytes().to_vec());
        AVWString {
            data: bytes.data.cast(),
            length: bytes.length,
        }
    }
}

impl ToAbi for String {
    type Abi = AVWString;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        let bytes = arena.array(self.as_bytes().to_vec());
        AVWString {
            data: bytes.data.cast(),
            length: bytes.length,
        }
    }
}

impl FromAbi for String {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        let array = AVWArray {
            data: raw.data.cast(),
            length: raw.length,
        };
        let bytes = array_values::<u8>(&array)?;
        Self::from_utf8(bytes.to_vec()).map_err(|error| {
            AVWriterError::InvalidState(format!("swift bridge returned invalid UTF-8: {error}"))
        })
    }
}

impl<T: ToAbi> ToAbi for [T] {
    type Abi = AVWArray;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        let values = self.iter().map(|value| value.to_abi(arena)).collect();
        arena.array(values)
    }
}

impl<T: ToAbi> ToAbi for Vec<T> {
    type Abi = AVWArray;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        let values = self.iter().map(|value| value.to_abi(arena)).collect();
        arena.array(values)
    }
}

impl<T: FromAbi> FromAbi for Vec<T> {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        array_values::<T::Abi>(raw)?
            .iter()
            .map(T::from_abi)
            .collect()
    }
}

impl<T: ToAbi> ToAbi for Option<T> {
    type Abi = *const T::Abi;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        self.as_ref().map_or(ptr::null(), |value| {
            let raw = value.to_abi(arena);
            arena.store(raw)
        })
    }
}

impl<T: FromAbi> FromAbi for Option<T> {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        unsafe { raw.as_ref() }.map(T::from_abi).transpose()
    }
}

impl ToAbi for bool {
    type Abi = u8;

    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        u8::from(*self)
    }
}

impl FromAbi for bool {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        match *raw {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(AVWriterError::InvalidState(
                "swift bridge returned an invalid boolean".into(),
            )),
        }
    }
}

impl ToAbi for u8 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for u8 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for i32 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for i32 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for u32 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for u32 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for i64 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for i64 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for u64 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for u64 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for usize {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for usize {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(*raw)
    }
}

impl ToAbi for f32 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for f32 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        if raw.is_finite() {
            Ok(*raw)
        } else {
            Err(AVWriterError::InvalidState(
                "invalid floating-point value".into(),
            ))
        }
    }
}

impl ToAbi for f64 {
    type Abi = Self;
    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        *self
    }
}

impl FromAbi for f64 {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        if raw.is_finite() {
            Ok(*raw)
        } else {
            Err(AVWriterError::InvalidState(
                "invalid floating-point value".into(),
            ))
        }
    }
}
