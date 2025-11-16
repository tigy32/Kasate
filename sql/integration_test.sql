-- Real PostgreSQL Integration Tests
-- These run actual SQL queries through the Kasate table access method

-- Step 1: Install the extension
CREATE EXTENSION IF NOT EXISTS kasate;

-- Step 2: Create a table using the Kasate table access method
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    price DECIMAL(10,2),
    stock INTEGER
) USING kasate;

-- Step 3: Insert data (goes through kasate_tuple_insert)
INSERT INTO products (name, price, stock) VALUES ('Widget', 19.99, 100);
INSERT INTO products (name, price, stock) VALUES ('Gadget', 29.99, 50);
INSERT INTO products (name, price, stock) VALUES ('Doohickey', 9.99, 200);
INSERT INTO products (name, price, stock) VALUES ('Thingamajig', 39.99, 75);

-- Step 4: Query data (goes through kasate_scan_begin, kasate_scan_getnextslot)
SELECT * FROM products;
SELECT name, price FROM products WHERE price > 15.00;
SELECT COUNT(*) FROM products;
SELECT SUM(stock) as total_stock FROM products;

-- Step 5: Update data (goes through kasate_tuple_update)
UPDATE products SET price = 24.99 WHERE name = 'Widget';
UPDATE products SET stock = stock - 10 WHERE name = 'Gadget';

-- Verify updates worked
SELECT name, price, stock FROM products WHERE name IN ('Widget', 'Gadget');

-- Step 6: Delete data (goes through kasate_tuple_delete)
DELETE FROM products WHERE name = 'Doohickey';

-- Verify delete worked
SELECT name FROM products;

-- Step 7: Test transactions and MVCC
BEGIN;
    INSERT INTO products (name, price, stock) VALUES ('Test Item', 1.00, 1);
    SELECT * FROM products WHERE name = 'Test Item'; -- Should see it
ROLLBACK;

-- After rollback, should not see it
SELECT * FROM products WHERE name = 'Test Item'; -- Should be empty

BEGIN;
    INSERT INTO products (name, price, stock) VALUES ('Committed Item', 2.00, 2);
COMMIT;

-- After commit, should see it
SELECT * FROM products WHERE name = 'Committed Item'; -- Should see it

-- Step 8: Test joins (uses scan operations)
CREATE TABLE orders (
    order_id SERIAL PRIMARY KEY,
    product_id INTEGER,
    quantity INTEGER
) USING kasate;

INSERT INTO orders (product_id, quantity) VALUES (1, 5);
INSERT INTO orders (product_id, quantity) VALUES (2, 3);
INSERT INTO orders (product_id, quantity) VALUES (1, 2);

-- Join across two Kasate tables
SELECT p.name, o.quantity
FROM products p
JOIN orders o ON p.id = o.product_id;

-- Step 9: Test aggregations (scan + aggregation)
SELECT
    AVG(price) as avg_price,
    MIN(price) as min_price,
    MAX(price) as max_price
FROM products;

-- Step 10: Test index operations (uses index_fetch callbacks)
CREATE INDEX idx_product_name ON products(name);
EXPLAIN SELECT * FROM products WHERE name = 'Widget';
SELECT * FROM products WHERE name = 'Widget';

-- Step 11: Verify storage statistics
SELECT * FROM kasate_storage_stats(
    (SELECT oid FROM pg_class WHERE relname = 'products')::oid
);

-- Step 12: Test with subqueries
SELECT name, price
FROM products
WHERE price > (SELECT AVG(price) FROM products);

-- Step 13: Test bulk insert
INSERT INTO products (name, price, stock)
SELECT
    'Bulk Item ' || i::text,
    (10 + random() * 40)::decimal(10,2),
    (random() * 100)::integer
FROM generate_series(1, 100) i;

-- Verify bulk insert
SELECT COUNT(*) FROM products;

-- Step 14: Test ordering (scan with sort)
SELECT name, price FROM products ORDER BY price DESC LIMIT 5;

-- Step 15: Test GROUP BY (scan with grouping)
SELECT
    CASE
        WHEN price < 15 THEN 'Cheap'
        WHEN price < 30 THEN 'Medium'
        ELSE 'Expensive'
    END as price_range,
    COUNT(*) as count
FROM products
GROUP BY price_range;

-- Cleanup
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;

-- Final status
SELECT 'All integration tests completed successfully!' as status;
