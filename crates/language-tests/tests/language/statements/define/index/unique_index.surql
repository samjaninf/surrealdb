/**
[test]

[[test.results]]
value = "NONE"

[[test.results]]
value = "[{ id: test:int, number: 0 }]"

[[test.results]]
value = "[{ id: test:float, number: 0f }]"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }]"

[[test.results]]
error = "Database index `index` already contains 0dec, with record `test:dec_int`"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }, { id: test:float, number: 0f }, { id: test:int, number: 0 }]"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }, { id: test:float, number: 0f }, { id: test:int, number: 0 }]"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }, { id: test:float, number: 0f }, { id: test:int, number: 0 }]"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }, { id: test:float, number: 0f }, { id: test:int, number: 0 }]"

[[test.results]]
value = "[{ id: test:dec_int, number: 0dec }, { id: test:float, number: 0f }, { id: test:int, number: 0 }]"

*/

DEFINE INDEX index ON TABLE test COLUMNS number UNIQUE;
CREATE test:int SET number = 0;
CREATE test:float SET number = 0.0;
CREATE test:dec_int SET number = 0dec;
CREATE test:dec_dec SET number = 0.0dec;
SELECT * FROM test WITH NOINDEX WHERE number = 0 ORDER BY id;
SELECT * FROM test WHERE number = 0 ORDER BY id;
SELECT * FROM test WHERE number = 0.0 ORDER BY id;
SELECT * FROM test WHERE number = 0dec ORDER BY id;
SELECT * FROM test WHERE number = 0.0dec ORDER BY id;
