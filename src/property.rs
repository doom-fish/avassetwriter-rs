#![allow(clippy::must_use_candidate, clippy::missing_const_for_fn)]

use std::collections::BTreeMap;

use crate::binary::{array_values, Arena, FromAbi, ToAbi};
use crate::error::AVWriterError;
use crate::ffi::{AVWArray, AVWPropertyEntry, AVWPropertyValue, AVWString};

#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum PropertyValue {
    #[default]
    Null,
    Boolean(bool),
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    String(String),
    Data(Vec<u8>),
    Array(Vec<Self>),
    Dictionary(BTreeMap<String, Self>),
}

impl PropertyValue {
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Dictionary(values) => values.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Unsigned(value) => i64::try_from(*value).ok(),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Unsigned(value) => Some(*value),
            Self::Integer(value) => u64::try_from(*value).ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_dictionary(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Dictionary(value) => Some(value),
            _ => None,
        }
    }
}

impl ToAbi for PropertyValue {
    type Abi = AVWPropertyValue;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        let empty = AVWArray {
            data: std::ptr::null(),
            length: 0,
        };
        let mut raw = AVWPropertyValue {
            kind: 0,
            boolean: 0,
            signed_value: 0,
            unsigned_value: 0,
            number: 0.0,
            string: AVWString {
                data: std::ptr::null(),
                length: 0,
            },
            bytes: empty,
            values: empty,
            entries: empty,
        };
        match self {
            Self::Null => {}
            Self::Boolean(value) => {
                raw.kind = 1;
                raw.boolean = u8::from(*value);
            }
            Self::Integer(value) => {
                raw.kind = 2;
                raw.signed_value = *value;
            }
            Self::Unsigned(value) => {
                raw.kind = 3;
                raw.unsigned_value = *value;
            }
            Self::Float(value) => {
                raw.kind = 4;
                raw.number = *value;
            }
            Self::String(value) => {
                raw.kind = 5;
                raw.string = value.to_abi(arena);
            }
            Self::Data(value) => {
                raw.kind = 6;
                raw.bytes = value.to_abi(arena);
            }
            Self::Array(value) => {
                raw.kind = 7;
                raw.values = value.to_abi(arena);
            }
            Self::Dictionary(value) => {
                raw.kind = 8;
                let entries = value
                    .iter()
                    .map(|(key, value)| AVWPropertyEntry {
                        key: key.to_abi(arena),
                        value: value.to_abi(arena),
                    })
                    .collect();
                raw.entries = arena.array(entries);
            }
        }
        raw
    }
}

impl FromAbi for PropertyValue {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        match raw.kind {
            0 => Ok(Self::Null),
            1 => Ok(Self::Boolean(bool::from_abi(&raw.boolean)?)),
            2 => Ok(Self::Integer(raw.signed_value)),
            3 => Ok(Self::Unsigned(raw.unsigned_value)),
            4 => Ok(Self::Float(f64::from_abi(&raw.number)?)),
            5 => Ok(Self::String(String::from_abi(&raw.string)?)),
            6 => Ok(Self::Data(Vec::<u8>::from_abi(&raw.bytes)?)),
            7 => Ok(Self::Array(Vec::<Self>::from_abi(&raw.values)?)),
            8 => {
                let entries = array_values::<AVWPropertyEntry>(&raw.entries)?
                    .iter()
                    .map(|entry| Ok((String::from_abi(&entry.key)?, Self::from_abi(&entry.value)?)))
                    .collect::<Result<_, AVWriterError>>()?;
                Ok(Self::Dictionary(entries))
            }
            _ => Err(AVWriterError::InvalidState(
                "swift bridge returned an invalid property value".into(),
            )),
        }
    }
}
