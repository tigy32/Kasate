-- Integration tests for Kasate table access method
-- Run these tests after installing the extension

-- Test 1: Create a table using Kasate
CREATE TABLE test_kasate_basic (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    value INTEGER
) USING kasate;

-- Test 2: Insert data
INSERT INTO test_kasate_basic (name, value) VALUES ('Alice', 100);
INSERT INTO test_kasate_basic (name, value) VALUES ('Bob', 200);
INSERT INTO test_kasate_basic (name, value) VALUES ('Charlie', 300);

-- Test 3: Select data
SELECT * FROM test_kasate_basic ORDER BY id;

-- Test 4: Update data
UPDATE test_kasate_basic SET value = 150 WHERE name = 'Alice';
SELECT * FROM test_kasate_basic WHERE name = 'Alice';

-- Test 5: Delete data
DELETE FROM test_kasate_basic WHERE name = 'Bob';
SELECT COUNT(*) FROM test_kasate_basic;

-- Test 6: Transaction support
BEGIN;
INSERT INTO test_kasate_basic (name, value) VALUES ('David', 400);
SELECT COUNT(*) FROM test_kasate_basic;
ROLLBACK;
SELECT COUNT(*) FROM test_kasate_basic; -- Should not include David

-- Test 7: Multiple inserts in transaction
BEGIN;
INSERT INTO test_kasate_basic (name, value) VALUES ('Eve', 500);
INSERT INTO test_kasate_basic (name, value) VALUES ('Frank', 600);
COMMIT;
SELECT COUNT(*) FROM test_kasate_basic;

-- Test 8: Test storage statistics
SELECT * FROM kasate_storage_stats('test_kasate_basic'::regclass);

-- Test 9: Create index
CREATE INDEX idx_kasate_name ON test_kasate_basic(name);
SELECT name FROM test_kasate_basic WHERE name = 'Alice';

-- Test 10: Aggregation queries
SELECT COUNT(*), SUM(value), AVG(value) FROM test_kasate_basic;

-- Test 11: Join operations
CREATE TABLE test_kasate_orders (
    order_id SERIAL PRIMARY KEY,
    customer_name TEXT NOT NULL,
    amount DECIMAL(10,2)
) USING kasate;

INSERT INTO test_kasate_orders (customer_name, amount) VALUES ('Alice', 50.00);
INSERT INTO test_kasate_orders (customer_name, amount) VALUES ('Charlie', 75.50);

SELECT b.name, b.value, o.amount
FROM test_kasate_basic b
JOIN test_kasate_orders o ON b.name = o.customer_name;

-- Test 12: Complex WHERE clauses
SELECT * FROM test_kasate_basic WHERE value > 200 AND name LIKE 'C%';

-- Test 13: Subqueries
SELECT name FROM test_kasate_basic
WHERE value > (SELECT AVG(value) FROM test_kasate_basic);

-- Test 14: NULL handling
ALTER TABLE test_kasate_basic ADD COLUMN description TEXT;
INSERT INTO test_kasate_basic (name, value, description) VALUES ('Grace', 700, NULL);
SELECT * FROM test_kasate_basic WHERE description IS NULL;

-- Test 15: Bulk insert
INSERT INTO test_kasate_basic (name, value)
SELECT 'User' || generate_series(1, 100), generate_series(1, 100) * 10;

SELECT COUNT(*) FROM test_kasate_basic;

-- Cleanup
DROP TABLE IF EXISTS test_kasate_orders;
DROP TABLE IF EXISTS test_kasate_basic;
