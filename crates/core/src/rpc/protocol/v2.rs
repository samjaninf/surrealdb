#[cfg(not(target_family = "wasm"))]
use async_graphql::BatchRequest;
use std::sync::Arc;

use crate::dbs::Variables;
#[cfg(not(target_family = "wasm"))]
use crate::dbs::capabilities::ExperimentalTarget;
use crate::err::Error;
use crate::expr::Object;
use crate::rpc::Data;
use crate::rpc::Method;
use crate::rpc::RpcContext;
use crate::rpc::RpcError;
use crate::rpc::statement_options::StatementOptions;
use crate::sql::Uuid;
use crate::{
	dbs::{QueryType, Response, capabilities::MethodTarget},
	expr::Value,
	rpc::args::Take,
	sql::{
		Array, Fields, Function, Model, Output, Query, SqlValue, Strand,
		statements::{
			CreateStatement, DeleteStatement, InsertStatement, KillStatement, LiveStatement,
			RelateStatement, SelectStatement, UpdateStatement, UpsertStatement,
		},
	},
};
use anyhow::Result;

#[expect(async_fn_in_trait)]
pub trait RpcProtocolV2: RpcContext {
	// ------------------------------
	// Method execution
	// ------------------------------

	/// Executes a method on this RPC implementation
	async fn execute(
		&self,
		_txn: Option<uuid::Uuid>,
		method: Method,
		params: Array,
	) -> Result<Data, RpcError> {
		// Check if capabilities allow executing the requested RPC method
		if !self.kvs().allows_rpc_method(&MethodTarget {
			method,
		}) {
			warn!("Capabilities denied RPC method call attempt, target: '{method}'");
			return Err(RpcError::MethodNotAllowed);
		}
		// Execute the desired method
		match method {
			Method::Ping => Ok(Value::None.into()),
			Method::Info => self.info().await,
			Method::Use => self.yuse(params).await,
			Method::Signup => self.signup(params).await,
			Method::Signin => self.signin(params).await,
			Method::Authenticate => self.authenticate(params).await,
			Method::Invalidate => self.invalidate().await,
			Method::Reset => self.reset().await,
			Method::Kill => self.kill(params).await,
			Method::Live => self.live(params).await,
			Method::Set => self.set(params).await,
			Method::Unset => self.unset(params).await,
			Method::Select => self.select(params).await,
			Method::Insert => self.insert(params).await,
			Method::Create => self.create(params).await,
			Method::Upsert => self.upsert(params).await,
			Method::Update => self.update(params).await,
			Method::Delete => self.delete(params).await,
			Method::Version => self.version(params).await,
			Method::Query => self.query(params).await,
			Method::Relate => self.relate(params).await,
			Method::Run => self.run(params).await,
			Method::GraphQL => self.graphql(params).await,
			_ => Err(RpcError::MethodNotFound),
		}
	}

	// ------------------------------
	// Methods for authentication
	// ------------------------------

	async fn yuse(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// For both ns+db, string = change, null = unset, none = do nothing
		// We need to be able to adjust either ns or db without affecting the other
		// To be able to select a namespace, and then list resources in that namespace, as an example
		let (ns, db) = params.needs_two()?;
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().as_ref().clone();
		// Update the selected namespace
		match ns {
			SqlValue::None => (),
			SqlValue::Null => session.ns = None,
			SqlValue::Strand(ns) => session.ns = Some(ns.0),
			_ => {
				return Err(RpcError::InvalidParams);
			}
		}
		// Update the selected database
		match db {
			SqlValue::None => (),
			SqlValue::Null => session.db = None,
			SqlValue::Strand(db) => session.db = Some(db.0),
			_ => {
				return Err(RpcError::InvalidParams);
			}
		}
		// Clear any residual database
		if self.session().ns.is_none() && self.session().db.is_some() {
			session.db = None;
		}
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return nothing
		Ok(Value::None.into())
	}

	async fn signup(&self, params: Array) -> Result<Data, RpcError> {
		// Process the method arguments
		let Ok(SqlValue::Object(params)) = params.needs_one() else {
			return Err(RpcError::InvalidParams);
		};
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().clone().as_ref().clone();
		// Attempt signup, mutating the session
		let out: Result<Value> =
			crate::iam::signup::signup(self.kvs(), &mut session, params.into())
				.await
				.map(Value::from);
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return the signup result
		out.map(Into::into).map_err(Into::into)
	}

	async fn signin(&self, params: Array) -> Result<Data, RpcError> {
		// Process the method arguments
		let Ok(SqlValue::Object(params)) = params.needs_one() else {
			return Err(RpcError::InvalidParams);
		};
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().clone().as_ref().clone();
		// Attempt signin, mutating the session
		let out: Result<Value> =
			crate::iam::signin::signin(self.kvs(), &mut session, params.into())
				.await
				.map(Value::from);
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return the signin result
		out.map(Into::into).map_err(Into::into)
	}

	async fn authenticate(&self, params: Array) -> Result<Data, RpcError> {
		// Process the method arguments
		let Ok(SqlValue::Strand(token)) = params.needs_one() else {
			return Err(RpcError::InvalidParams);
		};
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().as_ref().clone();
		// Attempt authentication, mutating the session
		let out: Result<Value> = crate::iam::verify::token(self.kvs(), &mut session, &token.0)
			.await
			.map(|_| Value::None);
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return nothing on success
		out.map_err(Into::into).map(Into::into)
	}

	async fn invalidate(&self) -> Result<Data, RpcError> {
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().as_ref().clone();
		// Clear the current session
		crate::iam::clear::clear(&mut session)?;
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return nothing on success
		Ok(Value::None.into())
	}

	async fn reset(&self) -> Result<Data, RpcError> {
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the current session
		let mut session = self.session().as_ref().clone();
		// Reset the current session
		crate::iam::reset::reset(&mut session);
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Cleanup live queries
		self.cleanup_lqs().await;
		// Return nothing on success
		Ok(Value::None.into())
	}

	// ------------------------------
	// Methods for identification
	// ------------------------------

	async fn info(&self) -> Result<Data, RpcError> {
		// Specify the SQL query string
		let sql = SelectStatement {
			expr: Fields::all(),
			what: vec![SqlValue::Param("auth".into())].into(),
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), None).await?;
		// Extract the first value from the result
		Ok(res.remove(0).result?.first().into())
	}

	// ------------------------------
	// Methods for setting variables
	// ------------------------------

	async fn set(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((SqlValue::Strand(key), val)) = params.needs_one_or_two() else {
			return Err(RpcError::InvalidParams);
		};
		// Specify the query parameters
		let var = Some(Variables::from(map! {
			key.0.clone() => Value::None,
		}));
		// Compute the specified parameter
		match self.kvs().compute(val.into(), &self.session(), var).await? {
			// Remove the variable if undefined
			Value::None => {
				// Get the context lock
				let mutex = self.lock().clone();
				// Lock the context for update
				let guard = mutex.acquire().await;
				// Clone the parameters
				let mut session = self.session().as_ref().clone();
				// Remove the set parameter
				session.variables.remove(&key.0);
				// Store the updated session
				self.set_session(Arc::new(session));
				// Drop the mutex guard
				std::mem::drop(guard);
			}
			// Store the variable if defined
			v => {
				// Get the context lock
				let mutex = self.lock().clone();
				// Lock the context for update
				let guard = mutex.acquire().await;
				// Clone the parameters
				let mut session = self.session().as_ref().clone();
				// Remove the set parameter
				session.variables.insert(key.0, v);
				// Store the updated session
				self.set_session(Arc::new(session));
				// Drop the mutex guard
				std::mem::drop(guard);
			}
		};
		// Return nothing
		Ok(Value::Null.into())
	}

	async fn unset(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok(SqlValue::Strand(key)) = params.needs_one() else {
			return Err(RpcError::InvalidParams);
		};
		// Get the context lock
		let mutex = self.lock().clone();
		// Lock the context for update
		let guard = mutex.acquire().await;
		// Clone the parameters
		let mut session = self.session().as_ref().clone();
		// Remove the set parameter
		session.variables.remove(&key.0);
		// Store the updated session
		self.set_session(Arc::new(session));
		// Drop the mutex guard
		std::mem::drop(guard);
		// Return nothing
		Ok(Value::Null.into())
	}

	// ------------------------------
	// Methods for live queries
	// ------------------------------

	async fn kill(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let id = params.needs_one()?;
		// Specify the SQL query string
		let sql = KillStatement {
			id,
		}
		.into();
		// Specify the query parameters
		let var = Some(self.session().variables.clone());
		// Execute the query on the database
		let mut res = self.query_inner(SqlValue::Query(sql), var).await?;
		// Extract the first query result
		Ok(res.remove(0).result?.into())
	}

	async fn live(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let (what, opts_value) = params.needs_one_or_two()?;
		// Prepare options
		let mut opts = StatementOptions::default();
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = LiveStatement {
			id: Uuid::new_v4(),
			node: Uuid::new_v4(),
			what: what.could_be_table(),
			expr: if opts.diff {
				Fields::default()
			} else {
				opts.fields.unwrap_or(Fields::all())
			},
			cond: opts.cond,
			fetch: opts.fetch,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.query_inner(SqlValue::Query(sql), var).await?;
		// Extract the first query result
		Ok(res.remove(0).result?.into())
	}

	// ------------------------------
	// Methods for selecting
	// ------------------------------

	async fn select(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, opts_value)) = params.needs_one_or_two() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = SelectStatement {
			only: opts.only,
			expr: opts.fields.unwrap_or_else(Fields::all),
			what: vec![what.could_be_table()].into(),
			start: opts.start,
			limit: opts.limit,
			cond: opts.cond,
			timeout: opts.timeout,
			version: opts.version,
			fetch: opts.fetch,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for inserting
	// ------------------------------

	async fn insert(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, data, opts_value)) = params.needs_two_or_three() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Insert data
		opts.with_data_content(data);
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Extract the data from the Option
		let Some(data) = opts.data_expr() else {
			return Err(RpcError::from(anyhow::Error::new(Error::unreachable(
				"Data content was previously set, so it cannot be Option::None",
			))));
		};
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = InsertStatement {
			into: match what.is_none_or_null() {
				false => Some(what.could_be_table()),
				true => None,
			},
			data,
			output: opts.output,
			relation: opts.relation,
			timeout: opts.timeout,
			version: opts.version,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for creating
	// ------------------------------

	async fn create(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, data, opts_value)) = params.needs_one_two_or_three() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Set the default output
		opts.with_output(Output::After);
		// Insert data
		if !data.is_none_or_null() {
			opts.with_data_content(data);
		}
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		let what = what.could_be_table();
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = CreateStatement {
			only: opts.only,
			what: vec![what.could_be_table()].into(),
			data: opts.data_expr(),
			output: opts.output,
			timeout: opts.timeout,
			version: opts.version,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for upserting
	// ------------------------------

	async fn upsert(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, data, opts_value)) = params.needs_one_two_or_three() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Set the default output
		opts.with_output(Output::After);
		// Insert data
		if !data.is_none_or_null() {
			opts.with_data_content(data);
		}
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = UpsertStatement {
			only: opts.only,
			what: vec![what.could_be_table()].into(),
			data: opts.data_expr(),
			output: opts.output,
			cond: opts.cond,
			timeout: opts.timeout,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for updating
	// ------------------------------

	async fn update(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, data, opts_value)) = params.needs_one_two_or_three() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Set the default output
		opts.with_output(Output::After);
		// Insert data
		if !data.is_none_or_null() {
			opts.with_data_content(data);
		}
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = UpdateStatement {
			only: opts.only,
			what: vec![what.could_be_table()].into(),
			data: opts.data_expr(),
			output: opts.output,
			cond: opts.cond,
			timeout: opts.timeout,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for relating
	// ------------------------------

	async fn relate(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((from, kind, with, data, opts_value)) = params.needs_three_four_or_five() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Set the default output
		opts.with_output(Output::After);
		// Insert data
		if !data.is_none_or_null() {
			opts.with_data_content(data);
		}
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = RelateStatement {
			only: opts.only,
			from,
			kind: kind.could_be_table(),
			with,
			data: opts.data_expr(),
			output: opts.output,
			timeout: opts.timeout,
			uniq: opts.unique,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for deleting
	// ------------------------------

	async fn delete(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((what, opts_value)) = params.needs_one_or_two() else {
			return Err(RpcError::InvalidParams);
		};
		// Prepare options
		let mut opts = StatementOptions::default();
		// Set the default output
		opts.with_output(Output::Before);
		// Apply user options
		if !opts_value.is_none_or_null() {
			opts.process_options(opts_value, self.kvs().get_capabilities())?;
		}
		// Specify the query parameters
		let var = Some(opts.merge_vars(&self.session().variables));
		// Specify the SQL query string
		let sql = DeleteStatement {
			only: opts.only,
			what: vec![what.could_be_table()].into(),
			output: opts.output,
			timeout: opts.timeout,
			cond: opts.cond,
			..Default::default()
		}
		.into();
		// Execute the query on the database
		let mut res = self.kvs().process(sql, &self.session(), var).await?;
		// Extract the first query result
		Ok(res
			.remove(0)
			.result
			.or_else(|e| match e.downcast_ref() {
				Some(Error::SingleOnlyOutput) => Ok(Value::None),
				_ => Err(RpcError::InternalError(e)),
			})?
			.into())
	}

	// ------------------------------
	// Methods for getting info
	// ------------------------------

	async fn version(&self, params: Array) -> Result<Data, RpcError> {
		match params.len() {
			0 => Ok(self.version_data()),
			_ => Err(RpcError::InvalidParams),
		}
	}

	// ------------------------------
	// Methods for querying
	// ------------------------------

	async fn query(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((query, vars)) = params.needs_one_or_two() else {
			return Err(RpcError::InvalidParams);
		};
		// Check the query input type
		if !(query.is_query() || query.is_strand()) {
			return Err(RpcError::InvalidParams);
		}
		// Specify the query variables
		let vars = match vars {
			SqlValue::Object(v) => {
				let v: Object = v.into();
				Some(self.session().variables.merged(v))
			}
			SqlValue::None | SqlValue::Null => Some(self.session().variables.clone()),
			_ => return Err(RpcError::InvalidParams),
		};
		// Execute the specified query
		self.query_inner(query, vars).await.map(Into::into)
	}

	// ------------------------------
	// Methods for running functions
	// ------------------------------

	async fn run(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		// Process the method arguments
		let Ok((name, version, args)) = params.needs_one_two_or_three() else {
			return Err(RpcError::InvalidParams);
		};
		// Parse the function name argument
		let name = match name {
			SqlValue::Strand(Strand(v)) => v,
			_ => return Err(RpcError::InvalidParams),
		};
		// Parse any function version argument
		let version = match version {
			SqlValue::Strand(Strand(v)) => Some(v),
			SqlValue::None | SqlValue::Null => None,
			_ => return Err(RpcError::InvalidParams),
		};
		// Parse the function arguments if specified
		let args = match args {
			SqlValue::Array(Array(arr)) => arr,
			SqlValue::None | SqlValue::Null => vec![],
			_ => return Err(RpcError::InvalidParams),
		};
		// Specify the function to run
		let func: Query = match &name[0..4] {
			"fn::" => Function::Custom(name.chars().skip(4).collect(), args).into(),
			"ml::" => Model {
				name: name.chars().skip(4).collect(),
				version: version.ok_or(RpcError::InvalidParams)?,
				args,
			}
			.into(),
			_ => Function::Normal(name, args).into(),
		};
		// Specify the query parameters
		let var = Some(self.session().variables.clone());
		// Execute the function on the database
		let mut res = self.kvs().process(func, &self.session(), var).await?;
		// Extract the first query result
		Ok(res.remove(0).result?.into())
	}

	// ------------------------------
	// Methods for querying with GraphQL
	// ------------------------------

	#[cfg(target_family = "wasm")]
	async fn graphql(&self, _: Array) -> Result<Data, RpcError> {
		Err(RpcError::MethodNotFound)
	}

	#[cfg(not(target_family = "wasm"))]
	async fn graphql(&self, params: Array) -> Result<Data, RpcError> {
		// Check if the user is allowed to query
		if !self.kvs().allows_query_by_subject(self.session().au.as_ref()) {
			return Err(RpcError::MethodNotAllowed);
		}
		if !self.kvs().get_capabilities().allows_experimental(&ExperimentalTarget::GraphQL) {
			return Err(RpcError::BadGQLConfig);
		}

		use serde::Serialize;

		use crate::gql;

		if !Self::GQL_SUPPORT {
			return Err(RpcError::BadGQLConfig);
		}

		let Ok((query, options)) = params.needs_one_or_two() else {
			return Err(RpcError::InvalidParams);
		};

		enum GraphQLFormat {
			Json,
		}

		// Default to compressed output
		let mut pretty = false;
		// Default to graphql json format
		let mut format = GraphQLFormat::Json;
		// Process any secondary config options
		match options {
			// A config object was passed
			SqlValue::Object(o) => {
				for (k, v) in o {
					match (k.as_str(), v) {
						("pretty", SqlValue::Bool(b)) => pretty = b,
						("format", SqlValue::Strand(s)) => match s.as_str() {
							"json" => format = GraphQLFormat::Json,
							_ => return Err(RpcError::InvalidParams),
						},
						_ => return Err(RpcError::InvalidParams),
					}
				}
			}
			// The config argument was not supplied
			SqlValue::None => (),
			// An invalid config argument was received
			_ => return Err(RpcError::InvalidParams),
		}
		// Process the graphql query argument
		let req = match query {
			// It is a string, so parse the query
			SqlValue::Strand(s) => match format {
				GraphQLFormat::Json => {
					let tmp: BatchRequest =
						serde_json::from_str(s.as_str()).map_err(|_| RpcError::ParseError)?;
					tmp.into_single().map_err(|_| RpcError::ParseError)?
				}
			},
			// It is an object, so build the query
			SqlValue::Object(mut o) => {
				// We expect a `query` key with the graphql query
				let mut tmp = match o.remove("query") {
					Some(SqlValue::Strand(s)) => async_graphql::Request::new(s),
					_ => return Err(RpcError::InvalidParams),
				};
				// We can accept a `variables` key with graphql variables
				match o.remove("variables").or(o.remove("vars")) {
					Some(obj @ SqlValue::Object(_)) => {
						let gql_vars = gql::schema::sql_value_to_gql_value(obj.into())
							.map_err(|_| RpcError::InvalidRequest)?;

						tmp = tmp.variables(async_graphql::Variables::from_value(gql_vars));
					}
					Some(_) => return Err(RpcError::InvalidParams),
					None => {}
				}
				// We can accept an `operation` key with a graphql operation name
				match o.remove("operationName").or(o.remove("operation")) {
					Some(SqlValue::Strand(s)) => tmp = tmp.operation_name(s),
					Some(_) => return Err(RpcError::InvalidParams),
					None => {}
				}
				// Return the graphql query object
				tmp
			}
			// We received an invalid graphql query
			_ => return Err(RpcError::InvalidParams),
		};
		// Process and cache the graphql schema
		let schema = self
			.graphql_schema_cache()
			.get_schema(&self.session())
			.await
			.map_err(|e| RpcError::Thrown(e.to_string()))?;
		// Execute the request against the schema
		let res = schema.execute(req).await;
		// Serialize the graphql response
		let out = if pretty {
			let mut buf = Vec::new();
			let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
			let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
			res.serialize(&mut ser).ok().and_then(|_| String::from_utf8(buf).ok())
		} else {
			serde_json::to_string(&res).ok()
		}
		.ok_or(RpcError::Thrown("Serialization Error".to_string()))?;
		// Output the graphql response
		Ok(Value::Strand(out.into()).into())
	}

	// ------------------------------
	// Private methods
	// ------------------------------

	async fn query_inner(
		&self,
		query: SqlValue,
		vars: Option<Variables>,
	) -> Result<Vec<Response>, RpcError> {
		// If no live query handler force realtime off
		if !Self::LQ_SUPPORT && self.session().rt {
			return Err(RpcError::BadLQConfig);
		}
		// Execute the query on the database
		let res = match query {
			SqlValue::Query(sql) => self.kvs().process(sql, &self.session(), vars).await?,
			SqlValue::Strand(sql) => self.kvs().execute(&sql, &self.session(), vars).await?,
			_ => {
				return Err(RpcError::from(anyhow::Error::new(Error::unreachable(
					"Unexpected query type: {query:?}",
				))));
			}
		};

		// Post-process hooks for web layer
		for response in &res {
			// This error should be unreachable because we shouldn't proceed if there's no handler
			self.handle_live_query_results(response).await;
		}
		// Return the result to the client
		Ok(res)
	}

	async fn handle_live_query_results(&self, res: &Response) {
		match &res.query_type {
			QueryType::Live => {
				if let Ok(Value::Uuid(lqid)) = &res.result {
					self.handle_live(&lqid.0).await;
				}
			}
			QueryType::Kill => {
				if let Ok(Value::Uuid(lqid)) = &res.result {
					self.handle_kill(&lqid.0).await;
				}
			}
			_ => {}
		}
	}
}
