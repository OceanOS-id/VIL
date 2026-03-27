CREATE TABLE IF NOT EXISTS tasks (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT DEFAULT '',
    done BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(100) DEFAULT '',
    price DECIMAL(10,2) DEFAULT 0,
    stock INTEGER DEFAULT 0
);

INSERT INTO products (name, category, price, stock) VALUES
    ('Laptop Pro', 'Electronics', 1299.99, 50),
    ('Wireless Mouse', 'Electronics', 29.99, 200),
    ('Standing Desk', 'Furniture', 599.00, 30),
    ('Monitor 4K', 'Electronics', 449.99, 75),
    ('Mechanical Keyboard', 'Electronics', 149.99, 120),
    ('Ergonomic Chair', 'Furniture', 899.00, 25),
    ('USB-C Hub', 'Electronics', 59.99, 300),
    ('Desk Lamp', 'Furniture', 79.99, 80);
