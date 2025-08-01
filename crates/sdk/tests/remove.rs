mod parse;
use parse::Parse;

mod helpers;
use helpers::*;
use surrealdb_core::iam::Level;

#[macro_use]
mod util;

use std::collections::HashMap;
use surrealdb::Result;
use surrealdb::dbs::Session;
use surrealdb::expr::Value;
use surrealdb::iam::Role;

#[tokio::test]
async fn remove_statement_table() -> Result<()> {
	let sql = "
		DEFINE TABLE test SCHEMALESS;
		REMOVE TABLE test;
		INFO FOR DB;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 3);
	//
	let tmp = res.remove(0).result;
	tmp.unwrap();
	//
	let tmp = res.remove(0).result;
	tmp.unwrap();
	//
	let tmp = res.remove(0).result?;
	let val = Value::parse(
		"{
			accesses: {},
			analyzers: {},
			apis: {},
			buckets: {},
			configs: {},
			functions: {},
			models: {},
			params: {},
			sequences: {},
			tables: {},
			users: {}
		}",
	);
	assert_eq!(tmp, val);
	Ok(())
}

#[tokio::test]
async fn remove_statement_namespace() -> Result<()> {
	// Namespace not selected
	{
		let sql = "
			REMOVE NAMESPACE test;
			DEFINE NAMESPACE test;
			REMOVE NAMESPACE test;
		";
		let dbs = new_ds().await?;
		let ses = Session::owner();
		let res = &mut dbs.execute(sql, &ses, None).await?;
		assert_eq!(res.len(), 3);
		//
		let tmp = res.remove(0).result;
		assert!(tmp.is_err());
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
	}
	// Namespace selected
	{
		let sql = "
			REMOVE NAMESPACE test;
			DEFINE NAMESPACE test;
			REMOVE NAMESPACE test;
		";
		let dbs = new_ds().await?;
		// No namespace is selected
		let ses = Session::owner().with_ns("test");
		let res = &mut dbs.execute(sql, &ses, None).await?;
		assert_eq!(res.len(), 3);
		//
		let tmp = res.remove(0).result;
		assert!(tmp.is_err());
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
	}
	Ok(())
}

#[tokio::test]
async fn remove_statement_database() -> Result<()> {
	// Database not selected
	{
		let sql = "
			REMOVE DATABASE test;
			DEFINE DATABASE test;
			REMOVE DATABASE test;
		";
		let dbs = new_ds().await?;
		let ses = Session::owner().with_ns("test");
		let res = &mut dbs.execute(sql, &ses, None).await?;
		assert_eq!(res.len(), 3);
		//
		let tmp = res.remove(0).result;
		assert!(tmp.is_err());
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
	}
	// Database selected
	{
		let sql = "
			REMOVE DATABASE test;
			DEFINE DATABASE test;
			REMOVE DATABASE test;
		";
		let dbs = new_ds().await?;
		// No database is selected
		let ses = Session::owner().with_ns("test").with_db("test");
		let res = &mut dbs.execute(sql, &ses, None).await?;
		assert_eq!(res.len(), 3);
		//
		let tmp = res.remove(0).result;
		assert!(tmp.is_err());
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
		//
		let tmp = res.remove(0).result;
		tmp.unwrap();
	}
	Ok(())
}

#[tokio::test]
async fn remove_statement_analyzer() -> Result<()> {
	let sql = "
		DEFINE ANALYZER english TOKENIZERS blank,class FILTERS lowercase,snowball(english);
		REMOVE ANALYZER english;
		INFO FOR DB;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 3);
	// Analyzer is defined
	let tmp = res.remove(0).result;
	tmp.unwrap();
	// Analyzer is removed
	let tmp = res.remove(0).result;
	tmp.unwrap();
	// Check infos output
	let tmp = res.remove(0).result?;
	let val = Value::parse(
		"{
			accesses: {},
			analyzers: {},
			apis: {},
			buckets: {},
			configs: {},
			functions: {},
			models: {},
			params: {},
			sequences: {},
			tables: {},
			users: {}
		}",
	);
	assert_eq!(tmp, val);
	Ok(())
}

#[tokio::test]
async fn remove_statement_index() -> Result<()> {
	let sql = "
		DEFINE INDEX uniq_isbn ON book FIELDS isbn UNIQUE;
		DEFINE INDEX idx_author ON book FIELDS author;
		DEFINE ANALYZER simple TOKENIZERS blank,class FILTERS lowercase;
		DEFINE INDEX ft_title ON book FIELDS title SEARCH ANALYZER simple BM25 HIGHLIGHTS;
		CREATE book:1 SET title = 'Rust Web Programming', isbn = '978-1803234694', author = 'Maxwell Flitton';
		REMOVE INDEX uniq_isbn ON book;
		REMOVE INDEX idx_author ON book;
		REMOVE INDEX ft_title ON book;
		INFO FOR TABLE book;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 9);
	for _ in 0..8 {
		let tmp = res.remove(0).result;
		tmp.unwrap();
	}
	// Check infos output
	let tmp = res.remove(0).result?;
	let val = Value::parse(
		"{
			events: {},
			fields: {},
			indexes: {},
			tables: {},
			lives: {},
		}",
	);
	assert_eq!(tmp, val);
	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_table_if_exists() -> Result<()> {
	let sql = "
		REMOVE TABLE IF EXISTS foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_analyzer_if_exists() -> Result<()> {
	let sql = "
		REMOVE ANALYZER IF EXISTS foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_database_if_exists() -> Result<()> {
	let sql = "
		REMOVE DATABASE IF EXISTS foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_event_if_exists() -> Result<()> {
	let sql = "
		REMOVE EVENT IF EXISTS foo ON bar;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_field_if_exists() -> Result<()> {
	let sql = "
		REMOVE FIELD IF EXISTS foo ON bar;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_function_if_exists() -> Result<()> {
	let sql = "
		REMOVE FUNCTION IF EXISTS fn::foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_index_if_exists() -> Result<()> {
	let sql = "
		REMOVE INDEX IF EXISTS foo ON bar;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_namespace_if_exists() -> Result<()> {
	let sql = "
		REMOVE NAMESPACE IF EXISTS foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_param_if_exists() -> Result<()> {
	let sql = "
		REMOVE PARAM IF EXISTS $foo;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_access_if_exists() -> Result<()> {
	let sql = "
		REMOVE ACCESS IF EXISTS foo ON DB;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

#[tokio::test]
async fn should_not_error_when_remove_user_if_exists() -> Result<()> {
	let sql = "
		REMOVE USER IF EXISTS foo ON ROOT;
	";
	let dbs = new_ds().await?;
	let ses = Session::owner().with_ns("test").with_db("test");
	let res = &mut dbs.execute(sql, &ses, None).await?;
	assert_eq!(res.len(), 1);
	//
	let tmp = res.remove(0).result?;
	assert_eq!(tmp, Value::None);

	Ok(())
}

//
// Permissions
//

fn level_root() -> Level {
	Level::Root
}
fn level_ns() -> Level {
	Level::Namespace("NS".to_owned())
}
fn level_db() -> Level {
	Level::Database("NS".to_owned(), "DB".to_owned())
}

#[tokio::test]
async fn permissions_checks_remove_ns() {
	let scenario = HashMap::from([
		("prepare", "DEFINE NAMESPACE NS"),
		("test", "REMOVE NAMESPACE NS"),
		("check", "INFO FOR ROOT"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check1 = "{ accesses: {  }, namespaces: {  }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: {  } }";
	let check2 = "{ accesses: {  }, namespaces: { NS: 'DEFINE NAMESPACE NS' }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: {  } }";
	let check_results = [vec![check1], vec![check2]];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), false),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_db() {
	let scenario = HashMap::from([
		("prepare", "DEFINE DATABASE DB"),
		("test", "REMOVE DATABASE DB"),
		("check", "INFO FOR NS"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ accesses: {  }, databases: {  }, users: {  } }"],
		vec!["{ accesses: {  }, databases: { DB: 'DEFINE DATABASE DB' }, users: {  } }"],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_function() {
	let scenario = HashMap::from([
		("prepare", "DEFINE FUNCTION fn::greet() {RETURN \"Hello\";}"),
		("test", "REMOVE FUNCTION fn::greet()"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: { greet: \"DEFINE FUNCTION fn::greet() { RETURN 'Hello'; } PERMISSIONS FULL\" }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_analyzer() {
	let scenario = HashMap::from([
		("prepare", "DEFINE ANALYZER analyzer TOKENIZERS BLANK"),
		("test", "REMOVE ANALYZER analyzer"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: {  }, analyzers: { analyzer: 'DEFINE ANALYZER analyzer TOKENIZERS BLANK' }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_root_access() {
	let scenario = HashMap::from([
		("prepare", "DEFINE ACCESS access ON ROOT TYPE JWT ALGORITHM HS512 KEY 'secret'"),
		("test", "REMOVE ACCESS access ON ROOT"),
		("check", "INFO FOR ROOT"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check1 = "{ accesses: {  }, namespaces: {  }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: {  } }";
	let check2 = r#"{ accesses: { access: "DEFINE ACCESS access ON ROOT TYPE JWT ALGORITHM HS512 KEY '[REDACTED]' WITH ISSUER KEY '[REDACTED]' DURATION FOR TOKEN 1h, FOR SESSION NONE" }, namespaces: {  }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: {  } }"#;
	let check_results = [vec![check1], vec![check2]];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), false),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_ns_access() {
	let scenario = HashMap::from([
		("prepare", "DEFINE ACCESS access ON NS TYPE JWT ALGORITHM HS512 KEY 'secret'"),
		("test", "REMOVE ACCESS access ON NS"),
		("check", "INFO FOR NS"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ accesses: {  }, databases: {  }, users: {  } }"],
		vec![
			"{ accesses: { access: \"DEFINE ACCESS access ON NAMESPACE TYPE JWT ALGORITHM HS512 KEY '[REDACTED]' WITH ISSUER KEY '[REDACTED]' DURATION FOR TOKEN 1h, FOR SESSION NONE\" }, databases: {  }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_db_access() {
	let scenario = HashMap::from([
		("prepare", "DEFINE ACCESS access ON DB TYPE JWT ALGORITHM HS512 KEY 'secret'"),
		("test", "REMOVE ACCESS access ON DB"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: { access: \"DEFINE ACCESS access ON DATABASE TYPE JWT ALGORITHM HS512 KEY '[REDACTED]' WITH ISSUER KEY '[REDACTED]' DURATION FOR TOKEN 1h, FOR SESSION NONE\" }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_root_user() {
	let scenario = HashMap::from([
		("prepare", "DEFINE USER user ON ROOT PASSHASH 'secret' ROLES VIEWER"),
		("test", "REMOVE USER user ON ROOT"),
		("check", "INFO FOR ROOT"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check1 = "{ accesses: {  }, namespaces: {  }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: {  } }";
	let check2 = r#"{ accesses: {  }, namespaces: {  }, nodes: {  }, system: { available_parallelism: 0, cpu_usage: 0.0f, load_average: [0.0f, 0.0f, 0.0f], memory_allocated: 0, memory_usage: 0, physical_cores: 0, threads: 0 }, users: { user: "DEFINE USER user ON ROOT PASSHASH 'secret' ROLES VIEWER DURATION FOR TOKEN 1h, FOR SESSION NONE" } }"#;
	let check_results = [vec![check1], vec![check2]];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), false),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_ns_user() {
	let scenario = HashMap::from([
		("prepare", "DEFINE USER user ON NS PASSHASH 'secret' ROLES VIEWER"),
		("test", "REMOVE USER user ON NS"),
		("check", "INFO FOR NS"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ accesses: {  }, databases: {  }, users: {  } }"],
		vec![
			"{ accesses: {  }, databases: {  }, users: { user: \"DEFINE USER user ON NAMESPACE PASSHASH 'secret' ROLES VIEWER DURATION FOR TOKEN 1h, FOR SESSION NONE\" } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), false),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_db_user() {
	let scenario = HashMap::from([
		("prepare", "DEFINE USER user ON DB PASSHASH 'secret' ROLES VIEWER"),
		("test", "REMOVE USER user ON DB"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: { user: \"DEFINE USER user ON DATABASE PASSHASH 'secret' ROLES VIEWER DURATION FOR TOKEN 1h, FOR SESSION NONE\" } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), false),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), false),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_param() {
	let scenario = HashMap::from([
		("prepare", "DEFINE PARAM $param VALUE 'foo'"),
		("test", "REMOVE PARAM $param"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: { param: \"DEFINE PARAM $param VALUE 'foo' PERMISSIONS FULL\" }, sequences: {  }, tables: {  }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_table() {
	let scenario = HashMap::from([
		("prepare", "DEFINE TABLE TB"),
		("test", "REMOVE TABLE TB"),
		("check", "INFO FOR DB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: {  }, users: {  } }",
		],
		vec![
			"{ accesses: {  }, analyzers: {  }, apis: {  }, buckets: {  }, configs: {  }, functions: {  }, models: {  }, params: {  }, sequences: {  }, tables: { TB: 'DEFINE TABLE TB TYPE ANY SCHEMALESS PERMISSIONS NONE' }, users: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_event() {
	let scenario = HashMap::from([
		("prepare", "DEFINE EVENT event ON TABLE TB WHEN true THEN RETURN 'foo'"),
		("test", "REMOVE EVENT event ON TABLE TB"),
		("check", "INFO FOR TABLE TB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ events: {  }, fields: {  }, indexes: {  }, lives: {  }, tables: {  } }"],
		vec![
			"{ events: { event: \"DEFINE EVENT event ON TB WHEN true THEN (RETURN 'foo')\" }, fields: {  }, indexes: {  }, lives: {  }, tables: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_field() {
	let scenario = HashMap::from([
		("prepare", "DEFINE FIELD field ON TABLE TB"),
		("test", "REMOVE FIELD field ON TABLE TB"),
		("check", "INFO FOR TABLE TB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ events: {  }, fields: {  }, indexes: {  }, lives: {  }, tables: {  } }"],
		vec![
			"{ events: {  }, fields: { field: 'DEFINE FIELD field ON TB PERMISSIONS FULL' }, indexes: {  }, lives: {  }, tables: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[tokio::test]
async fn permissions_checks_remove_index() {
	let scenario = HashMap::from([
		("prepare", "DEFINE INDEX index ON TABLE TB FIELDS field"),
		("test", "REMOVE INDEX index ON TABLE TB"),
		("check", "INFO FOR TABLE TB"),
	]);

	// Define the expected results for the check statement when the test statement succeeded and when it failed
	let check_results = [
		vec!["{ events: {  }, fields: {  }, indexes: {  }, lives: {  }, tables: {  } }"],
		vec![
			"{ events: {  }, fields: {  }, indexes: { index: 'DEFINE INDEX index ON TB FIELDS field' }, lives: {  }, tables: {  } }",
		],
	];

	let test_cases = [
		// Root level
		((level_root(), Role::Owner), ("NS", "DB"), true),
		((level_root(), Role::Editor), ("NS", "DB"), true),
		((level_root(), Role::Viewer), ("NS", "DB"), false),
		// Namespace level
		((level_ns(), Role::Owner), ("NS", "DB"), true),
		((level_ns(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Editor), ("NS", "DB"), true),
		((level_ns(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_ns(), Role::Viewer), ("NS", "DB"), false),
		((level_ns(), Role::Viewer), ("OTHER_NS", "DB"), false),
		// Database level
		((level_db(), Role::Owner), ("NS", "DB"), true),
		((level_db(), Role::Owner), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Owner), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Editor), ("NS", "DB"), true),
		((level_db(), Role::Editor), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Editor), ("OTHER_NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "DB"), false),
		((level_db(), Role::Viewer), ("NS", "OTHER_DB"), false),
		((level_db(), Role::Viewer), ("OTHER_NS", "DB"), false),
	];

	let res = iam_check_cases(test_cases.iter(), &scenario, check_results).await;
	assert!(res.is_ok(), "{}", res.unwrap_err());
}
