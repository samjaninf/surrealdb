use crate::ctx::Context;
use crate::dbs::Force;
use crate::dbs::Options;
use crate::dbs::Statement;
use crate::doc::{CursorDoc, Document};
use crate::err::Error;
use crate::expr::array::Array;
use crate::expr::index::{FullTextParams, HnswParams, Index, MTreeParams, SearchParams};
use crate::expr::statements::DefineIndexStatement;
use crate::expr::{FlowResultExt as _, Part, Thing, Value};
use crate::idx::IndexKeyBase;
use crate::idx::ft::fulltext::FullTextIndex;
use crate::idx::ft::search::SearchIndex;
use crate::idx::trees::mtree::MTreeIndex;
use crate::key;
#[cfg(not(target_family = "wasm"))]
use crate::kvs::ConsumeResult;
use crate::kvs::TransactionType;
use anyhow::{Result, bail};
use reblessive::tree::Stk;

impl Document {
	pub(super) async fn store_index_data(
		&self,
		stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		_stm: &Statement<'_>,
	) -> Result<()> {
		// Was this force targeted at a specific index?
		let targeted_force = matches!(opt.force, Force::Index(_));
		// Collect indexes or skip
		let ixs = match &opt.force {
			Force::Index(ix)
				if ix
					.first()
					.is_some_and(|ix| self.id.as_ref().is_some_and(|id| ix.what.0 == id.tb)) =>
			{
				ix.clone()
			}
			Force::All => self.ix(ctx, opt).await?,
			_ if self.changed() => self.ix(ctx, opt).await?,
			_ => return Ok(()),
		};
		// Check if the table is a view
		if self.tb(ctx, opt).await?.drop {
			return Ok(());
		}
		// Get the record id
		let rid = self.id()?;
		// Loop through all index statements
		for ix in ixs.iter() {
			// Calculate old values
			let o = Self::build_opt_values(stk, ctx, opt, ix, &self.initial).await?;

			// Calculate new values
			let n = Self::build_opt_values(stk, ctx, opt, ix, &self.current).await?;

			// Update the index entries
			if targeted_force || o != n {
				Self::one_index(stk, ctx, opt, ix, o, n, &rid).await?;
			}
		}
		// Carry on
		Ok(())
	}

	async fn one_index(
		stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		ix: &DefineIndexStatement,
		o: Option<Vec<Value>>,
		n: Option<Vec<Value>>,
		rid: &Thing,
	) -> Result<()> {
		#[cfg(not(target_family = "wasm"))]
		let (o, n) = if let Some(ib) = ctx.get_index_builder() {
			match ib.consume(ctx, opt.ns_db()?, ix, o, n, rid).await? {
				// The index builder consumed the value, which means it is currently building the index asynchronously,
				// we don't index the document and let the index builder do it later.
				ConsumeResult::Enqueued => return Ok(()),
				// The index builder is done, the index has been built; we can proceed normally
				ConsumeResult::Ignored(o, n) => (o, n),
			}
		} else {
			(o, n)
		};

		// Store all the variable and parameters required by the index operation
		let mut ic = IndexOperation::new(opt, ix, o, n, rid);

		// Index operation dispatching
		match &ix.index {
			Index::Uniq => ic.index_unique(ctx).await?,
			Index::Idx => ic.index_non_unique(ctx).await?,
			Index::Search(p) => ic.index_search(stk, ctx, p).await?,
			Index::FullText(p) => ic.index_fulltext(stk, ctx, p).await?,
			Index::MTree(p) => ic.index_mtree(stk, ctx, p).await?,
			Index::Hnsw(p) => ic.index_hnsw(ctx, p).await?,
		}
		Ok(())
	}

	/// Extract from the given document, the values required by the index and put then in an array.
	/// Eg. IF the index is composed of the columns `name` and `instrument`
	/// Given this doc: { "id": 1, "instrument":"piano", "name":"Tobie" }
	/// It will return: ["Tobie", "piano"]
	pub(crate) async fn build_opt_values(
		stk: &mut Stk,
		ctx: &Context,
		opt: &Options,
		ix: &DefineIndexStatement,
		doc: &CursorDoc,
	) -> Result<Option<Vec<Value>>> {
		if !doc.doc.as_ref().is_some() {
			return Ok(None);
		}
		let mut o = Vec::with_capacity(ix.cols.len());
		for i in ix.cols.iter() {
			let v = i.compute(stk, ctx, opt, Some(doc)).await.catch_return()?;
			o.push(v);
		}
		Ok(Some(o))
	}
}

/// Extract from the given document, the values required by the index and put then in an array.
/// Eg. IF the index is composed of the columns `name` and `instrument`
/// Given this doc: { "id": 1, "instrument":"piano", "name":"Tobie" }
/// It will return: ["Tobie", "piano"]
struct Indexable(Vec<(Value, bool)>);

impl Indexable {
	fn new(vals: Vec<Value>, ix: &DefineIndexStatement) -> Self {
		let mut source = Vec::with_capacity(vals.len());
		for (v, i) in vals.into_iter().zip(ix.cols.0.iter()) {
			let f = matches!(i.0.last(), Some(&Part::Flatten));
			source.push((v, f));
		}
		Self(source)
	}
}

impl IntoIterator for Indexable {
	type Item = Array;
	type IntoIter = Combinator;

	fn into_iter(self) -> Self::IntoIter {
		Combinator::new(self.0)
	}
}

struct Combinator {
	iterators: Vec<Box<dyn ValuesIterator>>,
	has_next: bool,
}

impl Combinator {
	fn new(source: Vec<(Value, bool)>) -> Self {
		let mut iterators: Vec<Box<dyn ValuesIterator>> = Vec::new();
		// We create an iterator for each idiom
		for (v, f) in source {
			if !f {
				// Iterator for not flattened values
				if let Value::Array(v) = v {
					iterators.push(Box::new(MultiValuesIterator::new(v.0)));
					continue;
				}
			}
			iterators.push(Box::new(SingleValueIterator(v)));
		}
		Self {
			iterators,
			has_next: true,
		}
	}
}

impl Iterator for Combinator {
	type Item = Array;

	fn next(&mut self) -> Option<Self::Item> {
		if !self.has_next {
			return None;
		}
		let mut o = Vec::with_capacity(self.iterators.len());
		// Create the combination and advance to the next
		self.has_next = false;
		for i in &mut self.iterators {
			o.push(i.current().clone());
			if !self.has_next {
				// We advance only one iterator per iteration
				if i.next() {
					self.has_next = true;
				}
			}
		}
		let o = Array::from(o);
		Some(o)
	}
}

trait ValuesIterator: Send {
	fn next(&mut self) -> bool;
	fn current(&self) -> &Value;
}

struct MultiValuesIterator {
	vals: Vec<Value>,
	done: bool,
	current: usize,
	end: usize,
}

impl MultiValuesIterator {
	fn new(vals: Vec<Value>) -> Self {
		let len = vals.len();
		if len == 0 {
			Self {
				vals,
				done: true,
				current: 0,
				end: 0,
			}
		} else {
			Self {
				vals,
				done: false,
				current: 0,
				end: len - 1,
			}
		}
	}
}

impl ValuesIterator for MultiValuesIterator {
	fn next(&mut self) -> bool {
		if self.done {
			return false;
		}
		if self.current == self.end {
			self.done = true;
			return false;
		}
		self.current += 1;
		true
	}

	fn current(&self) -> &Value {
		self.vals.get(self.current).unwrap_or(&Value::Null)
	}
}

struct SingleValueIterator(Value);

impl ValuesIterator for SingleValueIterator {
	fn next(&mut self) -> bool {
		false
	}

	fn current(&self) -> &Value {
		&self.0
	}
}

struct IndexOperation<'a> {
	opt: &'a Options,
	ix: &'a DefineIndexStatement,
	/// The old values (if existing)
	o: Option<Vec<Value>>,
	/// The new values (if existing)
	n: Option<Vec<Value>>,
	rid: &'a Thing,
}

impl<'a> IndexOperation<'a> {
	fn new(
		opt: &'a Options,
		ix: &'a DefineIndexStatement,
		o: Option<Vec<Value>>,
		n: Option<Vec<Value>>,
		rid: &'a Thing,
	) -> Self {
		Self {
			opt,
			ix,
			o,
			n,
			rid,
		}
	}

	fn get_unique_index_key(&self, v: &'a Array) -> Result<key::index::Index> {
		let (ns, db) = self.opt.ns_db()?;
		Ok(key::index::Index::new(ns, db, &self.ix.what, &self.ix.name, v, None))
	}

	fn get_non_unique_index_key(&self, v: &'a Array) -> Result<key::index::Index> {
		let (ns, db) = self.opt.ns_db()?;
		Ok(key::index::Index::new(ns, db, &self.ix.what, &self.ix.name, v, Some(&self.rid.id)))
	}

	async fn index_unique(&mut self, ctx: &Context) -> Result<()> {
		// Get the transaction
		let txn = ctx.tx();
		// Lock the transaction
		let mut txn = txn.lock().await;
		// Delete the old index data
		if let Some(o) = self.o.take() {
			let i = Indexable::new(o, self.ix);
			for o in i {
				let key = self.get_unique_index_key(&o)?;
				match txn.delc(&key, Some(self.rid)).await {
					Err(e) => {
						if matches!(e.downcast_ref(), Some(Error::TxConditionNotMet)) {
							Ok(())
						} else {
							Err(e)
						}
					}
					Ok(v) => Ok(v),
				}?
			}
		}
		// Create the new index data
		if let Some(n) = self.n.take() {
			let i = Indexable::new(n, self.ix);
			for n in i {
				if !n.is_all_none_or_null() {
					let key = self.get_unique_index_key(&n)?;
					if txn.putc(&key, self.rid, None).await.is_err() {
						let key = self.get_unique_index_key(&n)?;
						let rid = txn.get(&key, None).await?.unwrap();
						return self.err_index_exists(rid, n);
					}
				}
			}
		}
		Ok(())
	}

	async fn index_non_unique(&mut self, ctx: &Context) -> Result<()> {
		// Get the transaction
		let txn = ctx.tx();
		// Lock the transaction
		let mut txn = txn.lock().await;
		// Delete the old index data
		if let Some(o) = self.o.take() {
			let i = Indexable::new(o, self.ix);
			for o in i {
				let key = self.get_non_unique_index_key(&o)?;
				match txn.delc(&key, Some(self.rid)).await {
					Err(e) => {
						if matches!(e.downcast_ref(), Some(Error::TxConditionNotMet)) {
							Ok(())
						} else {
							Err(e)
						}
					}
					Ok(v) => Ok(v),
				}?
			}
		}
		// Create the new index data
		if let Some(n) = self.n.take() {
			let i = Indexable::new(n, self.ix);
			for n in i {
				let key = self.get_non_unique_index_key(&n)?;
				txn.set(&key, self.rid, None).await?;
			}
		}
		Ok(())
	}

	fn err_index_exists(&self, rid: Thing, n: Array) -> Result<()> {
		bail!(Error::IndexExists {
			thing: rid,
			index: self.ix.name.to_string(),
			value: match n.len() {
				1 => n.first().unwrap().to_string(),
				_ => n.to_string(),
			},
		})
	}

	async fn index_search(&mut self, stk: &mut Stk, ctx: &Context, p: &SearchParams) -> Result<()> {
		let (ns, db) = self.opt.ns_db()?;
		let ikb = IndexKeyBase::new(ns, db, &self.ix.what, &self.ix.name);

		let mut ft = SearchIndex::new(ctx, self.opt, &p.az, ikb, p, TransactionType::Write).await?;

		if let Some(n) = self.n.take() {
			ft.index_document(stk, ctx, self.opt, self.rid, n).await?;
		} else {
			ft.remove_document(ctx, self.rid).await?;
		}
		ft.finish(ctx).await
	}

	async fn index_fulltext(
		&mut self,
		stk: &mut Stk,
		ctx: &Context,
		p: &FullTextParams,
	) -> Result<()> {
		let (ns, db) = self.opt.ns_db()?;
		let ikb = IndexKeyBase::new(ns, db, &self.ix.what, &self.ix.name);
		let tx = ctx.tx();
		// Build a FullText instance
		let fti =
			FullTextIndex::new(self.opt.id()?, ctx.get_index_stores(), &tx, ikb.clone(), p).await?;
		let mut rc = false;
		// Delete the old index data
		let doc_id = if let Some(o) = self.o.take() {
			fti.remove_content(stk, ctx, self.opt, self.rid, o, &mut rc).await?
		} else {
			None
		};
		// Create the new index data
		if let Some(n) = self.n.take() {
			fti.index_content(stk, ctx, self.opt, self.rid, n, &mut rc).await?;
		} else if let Some(doc_id) = doc_id {
			fti.remove_doc(ctx, doc_id).await?;
		}
		// Do we need to trigger the compaction?
		if rc {
			FullTextIndex::trigger_compaction(&ikb, &tx, self.opt.id()?).await?;
		}
		Ok(())
	}

	async fn index_mtree(&mut self, stk: &mut Stk, ctx: &Context, p: &MTreeParams) -> Result<()> {
		let txn = ctx.tx();
		let (ns, db) = self.opt.ns_db()?;
		let ikb = IndexKeyBase::new(ns, db, &self.ix.what, &self.ix.name);
		let mut mt = MTreeIndex::new(&txn, ikb, p, TransactionType::Write).await?;
		// Delete the old index data
		if let Some(o) = self.o.take() {
			mt.remove_document(stk, &txn, self.rid, &o).await?;
		}
		// Create the new index data
		if let Some(n) = self.n.take() {
			mt.index_document(stk, &txn, self.rid, &n).await?;
		}
		mt.finish(&txn).await
	}

	async fn index_hnsw(&mut self, ctx: &Context, p: &HnswParams) -> Result<()> {
		let hnsw = ctx.get_index_stores().get_index_hnsw(ctx, self.opt, self.ix, p).await?;
		let mut hnsw = hnsw.write().await;
		// Delete the old index data
		if let Some(o) = self.o.take() {
			hnsw.remove_document(&ctx.tx(), self.rid.id.clone(), &o).await?;
		}
		// Create the new index data
		if let Some(n) = self.n.take() {
			hnsw.index_document(&ctx.tx(), &self.rid.id, &n).await?;
		}
		Ok(())
	}
}
