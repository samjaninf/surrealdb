use crate::ctx::Context;
#[cfg(target_family = "wasm")]
use crate::dbs::Force;
use crate::dbs::Options;
use crate::doc::CursorDoc;
use crate::err::Error;
use crate::expr::statements::DefineTableStatement;
use crate::expr::statements::info::InfoStructure;
#[cfg(target_family = "wasm")]
use crate::expr::statements::{RemoveIndexStatement, UpdateStatement};
use crate::expr::{Base, Ident, Idioms, Index, Part, Strand, Value};
#[cfg(target_family = "wasm")]
use crate::expr::{Output, Values};
use crate::iam::{Action, ResourceKind};
use crate::kvs::impl_kv_value_revisioned;
use anyhow::{Result, bail};
use reblessive::tree::Stk;
use revision::revisioned;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
#[cfg(target_family = "wasm")]
use std::sync::Arc;
use uuid::Uuid;

#[revisioned(revision = 4)]
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[non_exhaustive]
pub struct DefineIndexStatement {
	pub name: Ident,
	pub what: Ident,
	pub cols: Idioms,
	pub index: Index,
	pub comment: Option<Strand>,
	#[revision(start = 2)]
	pub if_not_exists: bool,
	#[revision(start = 3)]
	pub overwrite: bool,
	#[revision(start = 4)]
	pub concurrently: bool,
}

impl_kv_value_revisioned!(DefineIndexStatement);

impl DefineIndexStatement {
	/// Process this type returning a computed simple Value
	pub(crate) async fn compute(
		&self,
		stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		doc: Option<&CursorDoc>,
	) -> Result<Value> {
		// Allowed to run?
		opt.is_allowed(Action::Edit, ResourceKind::Index, &Base::Db)?;
		// Get the NS and DB
		let (ns, db) = opt.ns_db()?;
		// Fetch the transaction
		let txn = ctx.tx();
		// Check if the definition exists
		if txn.get_tb_index(ns, db, &self.what, &self.name).await.is_ok() {
			if self.if_not_exists {
				return Ok(Value::None);
			} else if !self.overwrite && !opt.import {
				bail!(Error::IxAlreadyExists {
					name: self.name.to_string(),
				});
			}
			// Clear the index store cache
			#[cfg(not(target_family = "wasm"))]
			ctx.get_index_stores()
				.index_removed(ctx.get_index_builder(), &txn, ns, db, &self.what, &self.name)
				.await?;
			#[cfg(target_family = "wasm")]
			ctx.get_index_stores().index_removed(&txn, ns, db, &self.what, &self.name).await?;
		}
		// Does the table exist?
		match txn.get_tb(ns, db, &self.what).await {
			Ok(tb) => {
				// Are we SchemaFull?
				if tb.full {
					// Check that the fields exist
					for idiom in self.cols.iter() {
						let Some(Part::Field(first)) = idiom.0.first() else {
							continue;
						};
						txn.get_tb_field(ns, db, &self.what, &first.to_string()).await?;
					}
				}
			}
			Err(e) => {
				if !matches!(e.downcast_ref(), Some(Error::TbNotFound { .. })) {
					return Err(e);
				}
			}
		}
		// Process the statement
		let key = crate::key::table::ix::new(ns, db, &self.what, &self.name);
		txn.get_or_add_ns(ns, opt.strict).await?;
		txn.get_or_add_db(ns, db, opt.strict).await?;
		txn.get_or_add_tb(ns, db, &self.what, opt.strict).await?;
		txn.set(
			&key,
			&DefineIndexStatement {
				// Don't persist the `IF NOT EXISTS`, `OVERWRITE` and `CONCURRENTLY` clause to schema
				if_not_exists: false,
				overwrite: false,
				concurrently: false,
				..self.clone()
			},
			None,
		)
		.await?;
		// Refresh the table cache
		let key = crate::key::database::tb::new(ns, db, &self.what);
		let tb = txn.get_tb(ns, db, &self.what).await?;
		txn.set(
			&key,
			&DefineTableStatement {
				cache_indexes_ts: Uuid::now_v7(),
				..tb.as_ref().clone()
			},
			None,
		)
		.await?;
		// Clear the cache
		if let Some(cache) = ctx.get_cache() {
			cache.clear_tb(ns, db, &self.what);
		}
		// Clear the cache
		txn.clear();
		// Process the index
		#[cfg(not(target_family = "wasm"))]
		self.async_index(stk, ctx, opt, doc, !self.concurrently).await?;
		#[cfg(target_family = "wasm")]
		self.sync_index(stk, ctx, opt, doc).await?;
		// Ok all good
		Ok(Value::None)
	}

	#[cfg(target_family = "wasm")]
	async fn sync_index(
		&self,
		stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		doc: Option<&CursorDoc>,
	) -> Result<()> {
		{
			// Create the remove statement
			let stm = RemoveIndexStatement {
				name: self.name.clone(),
				what: self.what.clone(),
				if_exists: false,
			};
			// Execute the delete statement
			stm.compute(ctx, opt).await?;
		}
		{
			// Force queries to run
			let opt = &opt.new_with_force(Force::Index(Arc::new([self.clone()])));
			// Update the index data
			let stm = UpdateStatement {
				what: Values(vec![Value::Table(self.what.clone().into())]),
				output: Some(Output::None),
				..UpdateStatement::default()
			};
			stm.compute(stk, ctx, opt, doc).await?;
		}
		Ok(())
	}

	#[cfg(not(target_family = "wasm"))]
	async fn async_index(
		&self,
		_stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		_doc: Option<&CursorDoc>,
		blocking: bool,
	) -> Result<()> {
		let rcv = ctx
			.get_index_builder()
			.ok_or_else(|| Error::unreachable("No Index Builder"))?
			.build(ctx, opt.clone(), self.clone().into(), blocking)?;
		if let Some(rcv) = rcv {
			rcv.await.map_err(|_| Error::IndexingBuildingCancelled)?
		} else {
			Ok(())
		}
	}
}

impl Display for DefineIndexStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE INDEX")?;
		if self.if_not_exists {
			write!(f, " IF NOT EXISTS")?
		}
		if self.overwrite {
			write!(f, " OVERWRITE")?
		}
		write!(f, " {} ON {} FIELDS {}", self.name, self.what, self.cols)?;
		if Index::Idx != self.index {
			write!(f, " {}", self.index)?;
		}
		if let Some(ref v) = self.comment {
			write!(f, " COMMENT {v}")?
		}
		if self.concurrently {
			write!(f, " CONCURRENTLY")?
		}
		Ok(())
	}
}

impl InfoStructure for DefineIndexStatement {
	fn structure(self) -> Value {
		Value::from(map! {
			"name".to_string() => self.name.structure(),
			"what".to_string() => self.what.structure(),
			"cols".to_string() => self.cols.structure(),
			"index".to_string() => self.index.structure(),
			"comment".to_string(), if let Some(v) = self.comment => v.into(),
		})
	}
}
