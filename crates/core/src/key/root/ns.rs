//! Stores a DEFINE NAMESPACE config definition
use crate::expr::statements::define::DefineNamespaceStatement;
use crate::key::category::Categorise;
use crate::key::category::Category;
use crate::kvs::KVKey;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub(crate) struct Ns<'a> {
	__: u8,
	_a: u8,
	_b: u8,
	_c: u8,
	pub ns: &'a str,
}

impl KVKey for Ns<'_> {
	type ValueType = DefineNamespaceStatement;
}

pub fn new(ns: &str) -> Ns<'_> {
	Ns::new(ns)
}

pub fn prefix() -> Vec<u8> {
	let mut k = super::all::kv();
	k.extend_from_slice(b"!ns\x00");
	k
}

pub fn suffix() -> Vec<u8> {
	let mut k = super::all::kv();
	k.extend_from_slice(b"!ns\xff");
	k
}

impl Categorise for Ns<'_> {
	fn categorise(&self) -> Category {
		Category::Namespace
	}
}

impl<'a> Ns<'a> {
	pub fn new(ns: &'a str) -> Self {
		Self {
			__: b'/',
			_a: b'!',
			_b: b'n',
			_c: b's',
			ns,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn key() {
		#[rustfmt::skip]
            let val = Ns::new(
            "testns",
        );
		let enc = Ns::encode_key(&val).unwrap();
		assert_eq!(enc, b"/!nstestns\0");
	}
}
