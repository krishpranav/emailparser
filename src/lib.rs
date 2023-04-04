#![forbid(unsafe_code)]

extern crate charset;
extern crate data_encoding;
extern crate quoted_printable;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::error;
use std::fmt;
use charset::{decode_latin1, Charset};

mod addrparse;
pub mod body;
mod dateparse;
mod header;
pub mod headers;
mod msgidparse;

use crate::body::Body;


#[derive(Debug)]
pub enum MailParseError {
    QuotedPrintableDecodeError(quoted_printable::QuotedPrintableError),
    Base64DecodeError(data_encoding::DecodeError),
    EncodingError(std::borrow::Cow<'static, str>),
    Generic(&'static str)
}

impl fmt::Display for MailParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MailParseError::QuotedPrintableDecodeError(ref err) => {
                write!(f, "QuotedPrintable decode error: {}", err)
            }
            MailParseError::Base64DecodeError(ref err) => write!(f, "Base64 decode error: {}", err),
            MailParseError::EncodingError(ref err) => write!(f, "Encoding Error: {}", err),
            MailParseError::Generic(ref description) => write!(f, "{}", description),
        }
    }
}

impl error::Error for MailParseError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            MailParseError::QuotedPrintableDecodeError(ref err) => Some(err),
            MailParseError::Base64DecodeError(ref err) => Some(err),
            _ => None,
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            MailParseError::QuotedPrintableDecodeError(ref err) => Some(err),
            MailParseError::Base64DecodeError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<quoted_printable::QuotedPrintableError> for MailParseError {
    fn from(err: quoted_printable::QuotedPrintableError) -> MailParseError {
        MailParseError::QuotedPrintableDecodeError(err)
    }
}

impl From<data_encoding::DecodeError> for MailParseError {
    fn from(err: data_encoding::DecodeError) -> MailParseError {
        MailParseError::Base64DecodeError(err)
    }
}

impl From<std::borrow::Cow<'static, str>> for MailParseError {
    fn from(err: std::borrow::Cow<'static, str>) -> MailParseError {
        MailParseError::EncodingError(err)
    }
}

pub struct MailHeader<'a> {
    key: &'a[u8],
    value: &'a[u8],
}

impl<'a> fmt::Debug for MailHeader<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MailHeader")
            .field("key", &String::from_utf8_lossy(self.key))
            .field("value", &String::from_utf8_lossy(self.value))
            .finish()
    }
}

// parser: find from email
pub(crate) fn find_from(line: &str, ix_start: usize, key: &str) -> Option<usize> {
    line[ix_start..].find(key).map(|v| ix_start + v)
}

fn find_from_u8(line: &[u8], ix_start: usize, key: &[u8]) -> Option<usize> {
    assert!(!key.is_empty());
    assert!(ix_start < line.len());
}