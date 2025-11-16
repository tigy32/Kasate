-- Basic usage examples for Kasate table access method

-- Example 1: Create a simple table
CREATE TABLE products (
    product_id SERIAL PRIMARY KEY,
    product_name TEXT NOT NULL,
    price DECIMAL(10,2),
    stock INTEGER DEFAULT 0
) USING kasate;

-- Insert some data
INSERT INTO products (product_name, price, stock) VALUES
    ('Widget', 19.99, 100),
    ('Gadget', 29.99, 50),
    ('Doohickey', 9.99, 200);

-- Query the data
SELECT * FROM products ORDER BY price;

-- Example 2: Using indexes
CREATE INDEX idx_product_name ON products(product_name);
CREATE INDEX idx_price ON products(price);

-- Query using index
EXPLAIN SELECT * FROM products WHERE product_name = 'Widget';
SELECT * FROM products WHERE product_name = 'Widget';

-- Example 3: Transactions
BEGIN;
INSERT INTO products (product_name, price, stock) VALUES ('Thingamajig', 14.99, 75);
UPDATE products SET stock = stock - 10 WHERE product_name = 'Widget';
SELECT * FROM products WHERE product_name IN ('Widget', 'Thingamajig');
COMMIT;

-- Example 4: Aggregations
SELECT
    COUNT(*) as total_products,
    AVG(price) as avg_price,
    SUM(stock) as total_stock
FROM products;

-- Example 5: Joins
CREATE TABLE orders (
    order_id SERIAL PRIMARY KEY,
    product_id INTEGER REFERENCES products(product_id),
    quantity INTEGER NOT NULL,
    order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) USING kasate;

INSERT INTO orders (product_id, quantity) VALUES
    (1, 5),
    (2, 3),
    (1, 2);

SELECT
    p.product_name,
    o.quantity,
    o.order_date
FROM products p
JOIN orders o ON p.product_id = o.product_id
ORDER BY o.order_date;

-- Example 6: Updates and Deletes
UPDATE products SET price = price * 1.1 WHERE stock < 100;
DELETE FROM products WHERE stock = 0;

SELECT * FROM products ORDER BY product_id;

-- Example 7: Check storage statistics
SELECT * FROM kasate_storage_stats('products'::regclass);
SELECT * FROM kasate_storage_stats('orders'::regclass);

-- Example 8: Subqueries
SELECT product_name, price
FROM products
WHERE price > (SELECT AVG(price) FROM products);

-- Example 9: Window functions
SELECT
    product_name,
    price,
    RANK() OVER (ORDER BY price DESC) as price_rank
FROM products;

-- Example 10: CTEs (Common Table Expressions)
WITH expensive_products AS (
    SELECT * FROM products WHERE price > 15.00
)
SELECT ep.product_name, o.quantity
FROM expensive_products ep
LEFT JOIN orders o ON ep.product_id = o.product_id;

-- Cleanup
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
