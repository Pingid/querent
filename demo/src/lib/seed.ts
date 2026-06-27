/**
 * Seed SQL executed once against the in-browser DuckDB instance. Builds a small
 * e-commerce dataset (customers, products, orders, order_items + a view) with
 * generated rows so the completion engine has a realistic schema to work with.
 */
export const DUCKDB_SEED = `
CREATE TABLE customers (
  id         INTEGER PRIMARY KEY,
  name       VARCHAR,
  email      VARCHAR,
  country    VARCHAR,
  created_at DATE
);

INSERT INTO customers VALUES
  (1,  'Ada Lovelace',     'ada@calc.io',       'UK',  DATE '2023-02-11'),
  (2,  'Alan Turing',      'alan@enigma.uk',    'UK',  DATE '2023-03-04'),
  (3,  'Grace Hopper',     'grace@navy.mil',    'US',  DATE '2023-04-21'),
  (4,  'Katherine Johnson','kj@nasa.gov',       'US',  DATE '2023-05-09'),
  (5,  'Hedy Lamarr',      'hedy@spread.at',    'AT',  DATE '2023-06-30'),
  (6,  'Dennis Ritchie',   'dmr@bell.com',      'US',  DATE '2023-07-17'),
  (7,  'Linus Torvalds',   'linus@kernel.org',  'FI',  DATE '2023-08-25'),
  (8,  'Margaret Hamilton','mh@apollo.gov',     'US',  DATE '2023-09-13'),
  (9,  'Tim Berners-Lee',  'tbl@web.org',       'UK',  DATE '2023-10-02'),
  (10, 'Barbara Liskov',   'liskov@mit.edu',    'US',  DATE '2023-11-19');

CREATE TABLE products (
  id       INTEGER PRIMARY KEY,
  name     VARCHAR,
  category VARCHAR,
  price    DECIMAL(10,2),
  in_stock BOOLEAN
);

INSERT INTO products VALUES
  (1, 'Mechanical Keyboard', 'Peripherals', 129.00, true),
  (2, 'Ultrawide Monitor',   'Displays',    549.00, true),
  (3, 'USB-C Hub',           'Accessories',  79.50, true),
  (4, 'Noise Cancel Headset','Audio',       299.00, false),
  (5, 'Webcam 4K',           'Peripherals', 159.00, true),
  (6, 'Standing Desk',       'Furniture',   699.00, true),
  (7, 'Ergonomic Chair',     'Furniture',   449.00, false),
  (8, 'Desk Lamp',           'Accessories',  39.90, true);

CREATE TABLE orders (
  id          INTEGER PRIMARY KEY,
  customer_id INTEGER,
  status      VARCHAR,
  total       DECIMAL(10,2),
  ordered_at  TIMESTAMP
);

INSERT INTO orders (id, customer_id, status, total, ordered_at)
SELECT
  i,
  1 + floor(random() * 10)::INT,
  (ARRAY['pending','paid','shipped','paid','shipped','refunded'])[1 + floor(random() * 6)::INT],
  0,
  TIMESTAMP '2024-01-01 00:00:00'
    + (floor(random() * 500)::INT * INTERVAL 1 DAY)
    + (floor(random() * 86400)::INT * INTERVAL 1 SECOND)
FROM range(1, 201) AS t(i);

CREATE TABLE order_items (
  order_id   INTEGER,
  product_id INTEGER,
  quantity   INTEGER,
  unit_price DECIMAL(10,2)
);

INSERT INTO order_items (order_id, product_id, quantity, unit_price)
SELECT o.order_id, p.id, o.quantity, p.price
FROM (
  SELECT
    1 + floor(random() * 200)::INT AS order_id,
    1 + floor(random() * 8)::INT   AS product_id,
    1 + floor(random() * 5)::INT   AS quantity
  FROM range(1, 401)
) o
JOIN products p ON p.id = o.product_id;

UPDATE orders SET total = COALESCE((
  SELECT round(sum(quantity * unit_price), 2)
  FROM order_items WHERE order_items.order_id = orders.id
), 0);

CREATE VIEW revenue_by_category AS
SELECT
  p.category,
  count(*)                              AS line_items,
  round(sum(oi.quantity * oi.unit_price), 2) AS revenue
FROM order_items oi
JOIN products p ON p.id = oi.product_id
GROUP BY p.category
ORDER BY revenue DESC;
`

/**
 * Equivalent dataset for SQLite. Differs from DuckDB in type names and row
 * generation: integers/reals/text instead of DECIMAL/BOOLEAN, recursive CTEs
 * instead of `range()`, and `random()`/`datetime()` for synthetic values.
 */
export const SQLITE_SEED = `
CREATE TABLE customers (
  id         INTEGER PRIMARY KEY,
  name       TEXT,
  email      TEXT,
  country    TEXT,
  created_at TEXT
);

INSERT INTO customers VALUES
  (1,  'Ada Lovelace',     'ada@calc.io',      'UK', '2023-02-11'),
  (2,  'Alan Turing',      'alan@enigma.uk',   'UK', '2023-03-04'),
  (3,  'Grace Hopper',     'grace@navy.mil',   'US', '2023-04-21'),
  (4,  'Katherine Johnson','kj@nasa.gov',      'US', '2023-05-09'),
  (5,  'Hedy Lamarr',      'hedy@spread.at',   'AT', '2023-06-30'),
  (6,  'Dennis Ritchie',   'dmr@bell.com',     'US', '2023-07-17'),
  (7,  'Linus Torvalds',   'linus@kernel.org', 'FI', '2023-08-25'),
  (8,  'Margaret Hamilton','mh@apollo.gov',    'US', '2023-09-13'),
  (9,  'Tim Berners-Lee',  'tbl@web.org',      'UK', '2023-10-02'),
  (10, 'Barbara Liskov',   'liskov@mit.edu',   'US', '2023-11-19');

CREATE TABLE products (
  id       INTEGER PRIMARY KEY,
  name     TEXT,
  category TEXT,
  price    REAL,
  in_stock INTEGER
);

INSERT INTO products VALUES
  (1, 'Mechanical Keyboard',  'Peripherals', 129.00, 1),
  (2, 'Ultrawide Monitor',    'Displays',    549.00, 1),
  (3, 'USB-C Hub',            'Accessories',  79.50, 1),
  (4, 'Noise Cancel Headset', 'Audio',       299.00, 0),
  (5, 'Webcam 4K',            'Peripherals', 159.00, 1),
  (6, 'Standing Desk',        'Furniture',   699.00, 1),
  (7, 'Ergonomic Chair',      'Furniture',   449.00, 0),
  (8, 'Desk Lamp',            'Accessories',  39.90, 1);

CREATE TABLE orders (
  id          INTEGER PRIMARY KEY,
  customer_id INTEGER,
  status      TEXT,
  total       REAL,
  ordered_at  TEXT
);

INSERT INTO orders (id, customer_id, status, total, ordered_at)
WITH RECURSIVE seq(i) AS (
  SELECT 1 UNION ALL SELECT i + 1 FROM seq WHERE i < 200
)
SELECT
  i,
  1 + abs(random() % 10),
  CASE abs(random() % 6)
    WHEN 0 THEN 'pending' WHEN 1 THEN 'paid' WHEN 2 THEN 'shipped'
    WHEN 3 THEN 'paid'    WHEN 4 THEN 'shipped' ELSE 'refunded'
  END,
  0,
  datetime('2024-01-01', '+' || abs(random() % 500) || ' days',
                         '+' || abs(random() % 86400) || ' seconds')
FROM seq;

CREATE TABLE order_items (
  order_id   INTEGER,
  product_id INTEGER,
  quantity   INTEGER,
  unit_price REAL
);

INSERT INTO order_items (order_id, product_id, quantity, unit_price)
SELECT o.order_id, p.id, o.quantity, p.price
FROM (
  WITH RECURSIVE seq(i) AS (
    SELECT 1 UNION ALL SELECT i + 1 FROM seq WHERE i < 400
  )
  SELECT
    1 + abs(random() % 200) AS order_id,
    1 + abs(random() % 8)   AS product_id,
    1 + abs(random() % 5)   AS quantity
  FROM seq
) o
JOIN products p ON p.id = o.product_id;

UPDATE orders SET total = COALESCE((
  SELECT round(sum(quantity * unit_price), 2)
  FROM order_items WHERE order_items.order_id = orders.id
), 0);

CREATE VIEW revenue_by_category AS
SELECT
  p.category,
  count(*)                                   AS line_items,
  round(sum(oi.quantity * oi.unit_price), 2) AS revenue
FROM order_items oi
JOIN products p ON p.id = oi.product_id
GROUP BY p.category
ORDER BY revenue DESC;
`

/**
 * Statements shown in the editor on first load. They build from a bare SELECT up
 * to a grouped join, each one exercising syntax the completion engine understands
 * (projections, WHERE, ORDER BY/LIMIT, JOIN, aggregates + GROUP BY). Run the
 * statement at the cursor with Cmd/Ctrl+Enter. The two dialects differ only where
 * the schema does: `in_stock` is BOOLEAN in DuckDB and INTEGER in SQLite.
 */
export const SQLITE_DEFAULT = `
-- select
SELECT * FROM products;

-- select, filter, order limit
SELECT name, price FROM products WHERE in_stock = 1 ORDER BY price DESC LIMIT 3;

-- joining
SELECT o.id, c.name, o.total FROM orders o JOIN customers c ON c.id = o.customer_id;

-- aggregate and group
SELECT status, count(*) AS orders, round(sum(total), 2) AS revenue
FROM orders GROUP BY status ORDER BY revenue DESC;

-- top customers by revenue
SELECT
  c.name,
  c.country,
  count(DISTINCT o.id)   AS orders,
  round(sum(o.total), 2) AS revenue
FROM customers c
JOIN orders o ON o.customer_id = c.id
WHERE o.status <> 'refunded'
GROUP BY c.name, c.country
ORDER BY revenue DESC
LIMIT 10;
`.trim()

/** See {@link SQLITE_DEFAULT}; identical progression with DuckDB's BOOLEAN `in_stock`. */
export const DUCKDB_DEFAULT = `
-- select
SELECT * FROM products;

-- select, filter, order limit
SELECT name, price FROM products WHERE in_stock = 1 ORDER BY price DESC LIMIT 3;

-- joining
SELECT o.id, c.name, o.total FROM orders o JOIN customers c ON c.id = o.customer_id;

-- aggregate and group
SELECT status, count(*) AS orders, round(sum(total), 2) AS revenue
FROM orders GROUP BY status ORDER BY revenue DESC;

-- top customers by revenue
SELECT
  c.name,
  c.country,
  count(DISTINCT o.id)   AS orders,
  round(sum(o.total), 2) AS revenue
FROM customers c
JOIN orders o ON o.customer_id = c.id
WHERE o.status <> 'refunded'
GROUP BY c.name, c.country
ORDER BY revenue DESC
LIMIT 10;
`.trim()
