use crate::ctx::Context;
use crate::dbs::Options;
use crate::doc::CursorDoc;
use crate::err::Error;
use crate::expr::statements::info::InfoStructure;
use crate::expr::{Base, Ident, Strand, Value};
use crate::iam::{Action, ResourceKind};
use crate::kvs::impl_kv_value_revisioned;
use anyhow::{Result, bail};

use revision::revisioned;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[revisioned(revision = 3)]
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[non_exhaustive]
pub struct DefineNamespaceStatement {
	pub id: Option<u32>,
	pub name: Ident,
	pub comment: Option<Strand>,
	#[revision(start = 2)]
	pub if_not_exists: bool,
	#[revision(start = 3)]
	pub overwrite: bool,
}

impl_kv_value_revisioned!(DefineNamespaceStatement);

impl DefineNamespaceStatement {
	/// Process this type returning a computed simple Value
	pub(crate) async fn compute(
		&self,
		ctx: &Context,
		opt: &Options,
		_doc: Option<&CursorDoc>,
	) -> Result<Value> {
		// Allowed to run?
		opt.is_allowed(Action::Edit, ResourceKind::Namespace, &Base::Root)?;
		// Fetch the transaction
		let txn = ctx.tx();
		// Check if the definition exists
		if txn.get_ns(&self.name).await.is_ok() {
			if self.if_not_exists {
				return Ok(Value::None);
			} else if !self.overwrite && !opt.import {
				bail!(Error::NsAlreadyExists {
					name: self.name.to_string(),
				});
			}
		}
		// Process the statement
		let key = crate::key::root::ns::new(&self.name);
		txn.set(
			&key,
			&DefineNamespaceStatement {
				id: match self.id {
					Some(id) => Some(id),
					None => Some(txn.lock().await.get_next_ns_id().await?),
				},
				// Don't persist the `IF NOT EXISTS` clause to schema
				if_not_exists: false,
				overwrite: false,
				..self.clone()
			},
			None,
		)
		.await?;
		// Clear the cache
		txn.clear();
		// Ok all good
		Ok(Value::None)
	}
}

impl Display for DefineNamespaceStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE NAMESPACE")?;
		if self.if_not_exists {
			write!(f, " IF NOT EXISTS")?
		}
		if self.overwrite {
			write!(f, " OVERWRITE")?
		}
		write!(f, " {}", self.name)?;
		if let Some(ref v) = self.comment {
			write!(f, " COMMENT {v}")?
		}
		Ok(())
	}
}

impl InfoStructure for DefineNamespaceStatement {
	fn structure(self) -> Value {
		Value::from(map! {
			"name".to_string() => self.name.structure(),
			"comment".to_string(), if let Some(v) = self.comment => v.into(),
		})
	}
}
