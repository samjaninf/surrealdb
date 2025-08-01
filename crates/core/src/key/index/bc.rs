//! Stores Doc list for each term
use crate::idx::ft::search::terms::TermId;
use crate::key::category::Categorise;
use crate::key::category::Category;
use crate::kvs::KVKey;

use roaring::RoaringTreemap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub(crate) struct Bc<'a> {
	__: u8,
	_a: u8,
	pub ns: &'a str,
	_b: u8,
	pub db: &'a str,
	_c: u8,
	pub tb: &'a str,
	_d: u8,
	pub ix: &'a str,
	_e: u8,
	_f: u8,
	_g: u8,
	pub term_id: TermId,
}

impl KVKey for Bc<'_> {
	type ValueType = RoaringTreemap;
}

impl Categorise for Bc<'_> {
	fn categorise(&self) -> Category {
		Category::IndexTermDocList
	}
}

impl<'a> Bc<'a> {
	pub fn new(ns: &'a str, db: &'a str, tb: &'a str, ix: &'a str, term_id: TermId) -> Self {
		Self {
			__: b'/',
			_a: b'*',
			ns,
			_b: b'*',
			db,
			_c: b'*',
			tb,
			_d: b'+',
			ix,
			_e: b'!',
			_f: b'b',
			_g: b'c',
			term_id,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn key() {
		#[rustfmt::skip]
		let val = Bc::new(
			"testns",
			"testdb",
			"testtb",
			"testix",
			7
		);
		let enc = Bc::encode_key(&val).unwrap();
		assert_eq!(enc, b"/*testns\0*testdb\0*testtb\0+testix\0!bc\0\0\0\0\0\0\0\x07");
	}
}
