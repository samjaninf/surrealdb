use crate::kvs::{Key, Val, impl_kv_value_revisioned};
use anyhow::Result;
use revision::revisioned;
use roaring::RoaringBitmap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// Id is a unique id generated by the generator.
pub(crate) type Id = u32;

// U64 is a generator that generates unique unsigned 64-bit integer ids.
// It can reuse freed ids by keeping track of them in a roaring bitmap.
// This doesn't do any variable-length encoding, so it's not as space efficient as it could be.
// It is used to generate ids for any SurrealDB objects that need aliases (e.g. namespaces, databases, tables, indexes, etc.)
#[revisioned(revision = 1)]
#[derive(Clone)]
#[non_exhaustive]
pub struct U32 {
	state_key: Key,
	available_ids: Option<RoaringBitmap>,
	next_id: Id,
	updated: bool,
}

impl_kv_value_revisioned!(U32);

impl U32 {
	pub(crate) async fn new(state_key: Key, v: Option<Val>) -> Result<Self> {
		let state: State = if let Some(val) = v {
			State::try_from_val(val)?
		} else {
			State::new()
		};
		Ok(Self {
			state_key,
			available_ids: state.available_ids,
			updated: false,
			next_id: state.next_id,
		})
	}

	pub(crate) fn get_next_id(&mut self) -> Id {
		self.updated = true;

		// We check first if there is any available id
		if let Some(available_ids) = &mut self.available_ids {
			if let Some(available_id) = available_ids.iter().next() {
				available_ids.remove(available_id);
				if available_ids.is_empty() {
					self.available_ids = None;
				}
				return available_id;
			}
		}
		// If not, we use the sequence
		let doc_id = self.next_id;
		self.next_id += 1;
		doc_id
	}

	pub(crate) fn remove_id(&mut self, id: Id) {
		if let Some(available_ids) = &mut self.available_ids {
			available_ids.insert(id);
		} else {
			let mut available_ids = RoaringBitmap::new();
			available_ids.insert(id);
			self.available_ids = Some(available_ids);
		}
		self.updated = true;
	}

	pub(crate) fn finish(&mut self) -> Option<(Key, Val)> {
		if self.updated {
			let state = State {
				available_ids: self.available_ids.take(),
				next_id: self.next_id,
			};
			return Some((self.state_key.clone(), state.try_to_val().unwrap()));
		}
		None
	}
}

#[derive(Serialize, Deserialize)]
struct State {
	available_ids: Option<RoaringBitmap>,
	next_id: Id,
}

impl State {
	fn new() -> Self {
		Self {
			available_ids: None,
			next_id: 0,
		}
	}
}

pub(crate) trait SerdeState
where
	Self: Sized + Serialize + DeserializeOwned,
{
	fn try_to_val(&self) -> Result<Val> {
		Ok(bincode::serialize(self)?)
	}

	fn try_from_val(val: Val) -> Result<Self> {
		Ok(bincode::deserialize(&val)?)
	}
}

impl SerdeState for RoaringBitmap {}
impl SerdeState for State {}

#[cfg(test)]
mod tests {
	use crate::idg::u32::U32;
	use crate::kvs::{Datastore, LockType::*, Transaction, TransactionType::*};
	use anyhow::Result;

	async fn get_ids(ds: &Datastore) -> (Transaction, U32) {
		let txn = ds.transaction(Write, Optimistic).await.unwrap();
		let key = "foo".as_bytes().to_vec();
		let v = txn.get(&key, None).await.unwrap();
		let d = U32::new(key, v).await.unwrap();
		(txn, d)
	}

	async fn finish(txn: Transaction, mut d: U32) -> Result<()> {
		if let Some((key, val)) = d.finish() {
			txn.set(&key, &val, None).await?;
		}
		txn.commit().await
	}

	#[tokio::test]
	async fn test_get_remove_ids() {
		let ds = Datastore::new("memory").await.unwrap();

		// Get the first id
		{
			let (tx, mut d) = get_ids(&ds).await;
			let id = d.get_next_id();
			finish(tx, d).await.unwrap();
			assert_eq!(id, 0);
		}

		// Get the second and the third ids
		{
			let (tx, mut d) = get_ids(&ds).await;
			let id1 = d.get_next_id();
			let id2 = d.get_next_id();
			finish(tx, d).await.unwrap();
			assert_eq!(id1, 1);
			assert_eq!(id2, 2);
		}

		// It reuses the removed id within a transaction
		{
			let (tx, mut d) = get_ids(&ds).await;
			d.remove_id(1);
			let id1 = d.get_next_id();
			let id2 = d.get_next_id();
			finish(tx, d).await.unwrap();
			assert_eq!(id1, 1);
			assert_eq!(id2, 3);
		}

		// It reuses the removed id across transactions
		{
			let (tx, mut d1) = get_ids(&ds).await;
			d1.remove_id(2);
			finish(tx, d1).await.unwrap();

			let (tx, mut d2) = get_ids(&ds).await;
			let id1 = d2.get_next_id();
			let id2 = d2.get_next_id();
			finish(tx, d2).await.unwrap();
			assert_eq!(id1, 2);
			assert_eq!(id2, 4);
		}
	}
}
