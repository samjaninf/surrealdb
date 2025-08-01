use crate::ctx::Context;
use crate::dbs::Options;
use crate::dbs::capabilities::ExperimentalTarget;
use crate::doc::CursorDoc;
use crate::err::Error;
use crate::expr::fmt::{is_pretty, pretty_indent};
use crate::expr::reference::Reference;
use crate::expr::statements::DefineTableStatement;
use crate::expr::statements::info::InfoStructure;
use crate::expr::{Base, Ident, Idiom, Kind, Permissions, Strand, Value};
use crate::expr::{Literal, Part};
use crate::expr::{Relation, TableType};
use crate::iam::{Action, ResourceKind};
use crate::kvs::{Transaction, impl_kv_value_revisioned};
use anyhow::{Result, bail, ensure};

use revision::revisioned;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Write};
use std::sync::Arc;
use uuid::Uuid;

#[revisioned(revision = 6)]
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[non_exhaustive]
pub struct DefineFieldStatement {
	pub name: Idiom,
	pub what: Ident,
	/// Whether the field is marked as flexible.
	/// Flexible allows the field to be schemaless even if the table is marked as schemafull.
	pub flex: bool,
	pub kind: Option<Kind>,
	#[revision(start = 2)]
	pub readonly: bool,
	pub value: Option<Value>,
	pub assert: Option<Value>,
	pub default: Option<Value>,
	pub permissions: Permissions,
	pub comment: Option<Strand>,
	#[revision(start = 3)]
	pub if_not_exists: bool,
	#[revision(start = 4)]
	pub overwrite: bool,
	#[revision(start = 5)]
	pub reference: Option<Reference>,
	#[revision(start = 6)]
	pub default_always: bool,
}

impl_kv_value_revisioned!(DefineFieldStatement);

impl DefineFieldStatement {
	/// Process this type returning a computed simple Value
	pub(crate) async fn compute(
		&self,
		ctx: &Context,
		opt: &Options,
		_doc: Option<&CursorDoc>,
	) -> Result<Value> {
		// Allowed to run?
		opt.is_allowed(Action::Edit, ResourceKind::Field, &Base::Db)?;
		// Validate reference options
		self.validate_reference_options(ctx)?;
		// Correct reference type
		let kind = if let Some(kind) = self.get_reference_kind(ctx, opt).await? {
			Some(kind)
		} else {
			self.kind.clone()
		};

		// Get the NS and DB
		let (ns, db) = opt.ns_db()?;

		// Disallow mismatched types
		self.disallow_mismatched_types(ctx, ns, db).await?;

		// Fetch the transaction
		let txn = ctx.tx();
		// Get the name of the field
		let fd = self.name.to_string();
		// Check if the definition exists
		if txn.get_tb_field(ns, db, &self.what, &fd).await.is_ok() {
			if self.if_not_exists {
				return Ok(Value::None);
			} else if !self.overwrite && !opt.import {
				bail!(Error::FdAlreadyExists {
					name: fd,
				});
			}
		}
		// Process the statement
		let key = crate::key::table::fd::new(ns, db, &self.what, &fd);
		txn.get_or_add_ns(ns, opt.strict).await?;
		txn.get_or_add_db(ns, db, opt.strict).await?;
		txn.get_or_add_tb(ns, db, &self.what, opt.strict).await?;
		txn.set(
			&key,
			&DefineFieldStatement {
				// Don't persist the `IF NOT EXISTS` clause to schema
				if_not_exists: false,
				overwrite: false,
				kind,
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
				cache_fields_ts: Uuid::now_v7(),
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
		// Process possible recursive defitions
		self.process_recursive_definitions(ns, db, txn.clone()).await?;
		// If this is an `in` field then check relation definitions
		if fd.as_str() == "in" {
			// Get the table definition that this field belongs to
			let tb = txn.get_tb(ns, db, &self.what).await?;
			// The table is marked as TYPE RELATION
			if let TableType::Relation(ref relation) = tb.kind {
				// Check if a field TYPE has been specified
				if let Some(kind) = self.kind.as_ref() {
					// The `in` field must be a record type
					ensure!(
						kind.is_record(),
						Error::Thrown("in field on a relation must be a record".into(),)
					);
					// Add the TYPE to the DEFINE TABLE statement
					if relation.from.as_ref() != self.kind.as_ref() {
						let key = crate::key::database::tb::new(ns, db, &self.what);
						let val = DefineTableStatement {
							cache_fields_ts: Uuid::now_v7(),
							kind: TableType::Relation(Relation {
								from: self.kind.clone(),
								..relation.to_owned()
							}),
							..tb.as_ref().to_owned()
						};
						txn.set(&key, &val, None).await?;
						// Clear the cache
						if let Some(cache) = ctx.get_cache() {
							cache.clear_tb(ns, db, &self.what);
						}
						// Clear the cache
						txn.clear();
					}
				}
			}
		}
		// If this is an `out` field then check relation definitions
		if fd.as_str() == "out" {
			// Get the table definition that this field belongs to
			let tb = txn.get_tb(ns, db, &self.what).await?;
			// The table is marked as TYPE RELATION
			if let TableType::Relation(ref relation) = tb.kind {
				// Check if a field TYPE has been specified
				if let Some(kind) = self.kind.as_ref() {
					// The `out` field must be a record type
					ensure!(
						kind.is_record(),
						Error::Thrown("out field on a relation must be a record".into(),)
					);
					// Add the TYPE to the DEFINE TABLE statement
					if relation.from.as_ref() != self.kind.as_ref() {
						let key = crate::key::database::tb::new(ns, db, &self.what);
						let val = DefineTableStatement {
							cache_fields_ts: Uuid::now_v7(),
							kind: TableType::Relation(Relation {
								to: self.kind.clone(),
								..relation.to_owned()
							}),
							..tb.as_ref().to_owned()
						};
						txn.set(&key, &val, None).await?;
						// Clear the cache
						if let Some(cache) = ctx.get_cache() {
							cache.clear_tb(ns, db, &self.what);
						}
						// Clear the cache
						txn.clear();
					}
				}
			}
		}
		// Clear the cache
		txn.clear();
		// Ok all good
		Ok(Value::None)
	}

	pub(crate) async fn process_recursive_definitions(
		&self,
		ns: &str,
		db: &str,
		txn: Arc<Transaction>,
	) -> Result<()> {
		// Find all existing field definitions
		let fields = txn.all_tb_fields(ns, db, &self.what, None).await.ok();
		// Process possible recursive_definitions
		if let Some(mut cur_kind) = self.kind.as_ref().and_then(|x| x.inner_kind()) {
			let mut name = self.name.clone();
			loop {
				// Check if the subtype is an `any` type
				if let Kind::Any = cur_kind {
					// There is no need to add a subtype
					// field definition if the type is
					// just specified as an `array`. This
					// is because the following query:
					//  DEFINE FIELD foo ON bar TYPE array;
					// already implies that the immediate
					// subtype is an any:
					//  DEFINE FIELD foo[*] ON bar TYPE any;
					// so we skip the subtype field.
					break;
				}
				// Get the kind of this sub field
				let new_kind = cur_kind.inner_kind();
				// Add a new subtype
				name.0.push(Part::All);
				// Get the field name
				let fd = name.to_string();
				// Set the subtype `DEFINE FIELD` definition
				let key = crate::key::table::fd::new(ns, db, &self.what, &fd);
				let val = if let Some(existing) =
					fields.as_ref().and_then(|x| x.iter().find(|x| x.name == name))
				{
					DefineFieldStatement {
						kind: Some(cur_kind),
						reference: self.reference.clone(),
						if_not_exists: false,
						overwrite: false,
						..existing.clone()
					}
				} else {
					DefineFieldStatement {
						name: name.clone(),
						what: self.what.clone(),
						flex: self.flex,
						kind: Some(cur_kind),
						reference: self.reference.clone(),
						..Default::default()
					}
				};
				txn.set(&key, &val, None).await?;
				// Process to any sub field
				if let Some(new_kind) = new_kind {
					cur_kind = new_kind;
				} else {
					break;
				}
			}
		}

		Ok(())
	}

	pub(crate) fn validate_reference_options(&self, ctx: &Context) -> Result<()> {
		if !ctx.get_capabilities().allows_experimental(&ExperimentalTarget::RecordReferences) {
			return Ok(());
		}

		if let Some(kind) = &self.kind {
			let kinds = match kind {
				Kind::Either(kinds) => kinds,
				kind => &vec![kind.to_owned()],
			};

			// Check if any of the kinds are references
			if kinds.iter().any(|k| matches!(k, Kind::References(_, _))) {
				// If any of the kinds are references, all of them must be
				ensure!(
					kinds.iter().all(|k| matches!(k, Kind::References(_, _))),
					Error::RefsMismatchingVariants
				);

				// As the refs and dynrefs type essentially take over a field
				// they are not allowed to be mixed with most other clauses
				let typename = kind.to_string();

				ensure!(
					self.reference.is_none(),
					Error::RefsTypeConflict("REFERENCE".into(), typename)
				);

				ensure!(
					self.default.is_none(),
					Error::RefsTypeConflict("DEFAULT".into(), typename)
				);

				ensure!(self.value.is_none(), Error::RefsTypeConflict("VALUE".into(), typename));

				ensure!(self.assert.is_none(), Error::RefsTypeConflict("ASSERT".into(), typename));

				ensure!(!self.flex, Error::RefsTypeConflict("FLEXIBLE".into(), typename));

				ensure!(!self.readonly, Error::RefsTypeConflict("READONLY".into(), typename));
			}

			// If a reference is defined, the field must be a record
			if self.reference.is_some() {
				let kinds = match kind.get_optional_inner_kind() {
					Kind::Either(kinds) => kinds,
					Kind::Array(kind, _) | Kind::Set(kind, _) => match kind.as_ref() {
						Kind::Either(kinds) => kinds,
						kind => &vec![kind.to_owned()],
					},
					Kind::Literal(lit) => match lit {
						Literal::Array(kinds) => kinds,
						lit => &vec![Kind::Literal(lit.to_owned())],
					},
					kind => &vec![kind.to_owned()],
				};

				ensure!(
					kinds.iter().all(|k| matches!(k, Kind::Record(_))),
					Error::ReferenceTypeConflict(kind.to_string())
				);
			}
		}

		Ok(())
	}

	/// Get the correct reference type if needed.
	pub(crate) async fn get_reference_kind(
		&self,
		ctx: &Context,
		opt: &Options,
	) -> Result<Option<Kind>> {
		if !ctx.get_capabilities().allows_experimental(&ExperimentalTarget::RecordReferences) {
			return Ok(None);
		}

		if let Some(Kind::References(Some(ft), Some(ff))) = &self.kind {
			// Obtain the field definition
			let (ns, db) = opt.ns_db()?;
			let fd = match ctx.tx().get_tb_field(ns, db, &ft.to_string(), &ff.to_string()).await {
				Ok(fd) => fd,
				// If the field does not exist, there is nothing to correct
				Err(e) => {
					if matches!(e.downcast_ref(), Some(Error::FdNotFound { .. })) {
						return Ok(None);
					} else {
						return Err(e);
					}
				}
			};

			// Check if the field is an array-like value and thus "containing" references
			let is_array_like = fd
				.kind
				.as_ref()
				.map(|kind| kind.get_optional_inner_kind().is_array_like())
				.unwrap_or_default();

			// If the field is an array-like value, add the `.*` part
			if is_array_like {
				let ff = ff.clone().push(Part::All);
				return Ok(Some(Kind::References(Some(ft.clone()), Some(ff))));
			}
		}

		Ok(None)
	}

	pub(crate) async fn disallow_mismatched_types(
		&self,
		ctx: &Context,
		ns: &str,
		db: &str,
	) -> Result<()> {
		let fds = ctx.tx().all_tb_fields(ns, db, &self.what, None).await?;

		if let Some(self_kind) = &self.kind {
			for fd in fds.iter() {
				if self.name.starts_with(&fd.name) && self.name != fd.name {
					if let Some(fd_kind) = &fd.kind {
						let path = self.name[fd.name.len()..].to_vec();
						if !fd_kind.allows_nested_kind(&path, self_kind) {
							bail!(Error::MismatchedFieldTypes {
								name: self.name.to_string(),
								kind: self_kind.to_string(),
								existing_name: fd.name.to_string(),
								existing_kind: fd_kind.to_string(),
							});
						}
					}
				}
			}
		}

		Ok(())
	}
}

impl Display for DefineFieldStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE FIELD")?;
		if self.if_not_exists {
			write!(f, " IF NOT EXISTS")?
		}
		if self.overwrite {
			write!(f, " OVERWRITE")?
		}
		write!(f, " {} ON {}", self.name, self.what)?;
		if self.flex {
			write!(f, " FLEXIBLE")?
		}
		if let Some(ref v) = self.kind {
			write!(f, " TYPE {v}")?
		}
		if let Some(ref v) = self.default {
			write!(f, " DEFAULT")?;
			if self.default_always {
				write!(f, " ALWAYS")?
			}

			write!(f, " {v}")?
		}
		if self.readonly {
			write!(f, " READONLY")?
		}
		if let Some(ref v) = self.value {
			write!(f, " VALUE {v}")?
		}
		if let Some(ref v) = self.assert {
			write!(f, " ASSERT {v}")?
		}
		if let Some(ref v) = self.reference {
			write!(f, " REFERENCE {v}")?
		}
		if let Some(ref v) = self.comment {
			write!(f, " COMMENT {v}")?
		}
		let _indent = if is_pretty() {
			Some(pretty_indent())
		} else {
			f.write_char(' ')?;
			None
		};
		// Alternate permissions display implementation ignores delete permission
		// This display is used to show field permissions, where delete has no effect
		// Displaying the permission could mislead users into thinking it has an effect
		// Additionally, including the permission will cause a parsing error in 3.0.0
		write!(f, "{:#}", self.permissions)?;
		Ok(())
	}
}

impl InfoStructure for DefineFieldStatement {
	fn structure(self) -> Value {
		Value::from(map! {
			"name".to_string() => self.name.structure(),
			"what".to_string() => self.what.structure(),
			"flex".to_string() => self.flex.into(),
			"kind".to_string(), if let Some(v) = self.kind => v.structure(),
			"value".to_string(), if let Some(v) = self.value => v.structure(),
			"assert".to_string(), if let Some(v) = self.assert => v.structure(),
			"default_always".to_string(), if self.default.is_some() => self.default_always.into(), // Only reported if DEFAULT is also enabled for this field
			"default".to_string(), if let Some(v) = self.default => v.structure(),
			"reference".to_string(), if let Some(v) = self.reference => v.structure(),
			"readonly".to_string() => self.readonly.into(),
			"permissions".to_string() => self.permissions.structure(),
			"comment".to_string(), if let Some(v) = self.comment => v.into(),
		})
	}
}
