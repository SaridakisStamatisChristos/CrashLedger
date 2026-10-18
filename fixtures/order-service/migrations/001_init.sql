-- No unique constraint on orders.idempotency_key: the broken mode MUST duplicate it.
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY,
    idempotency_key TEXT NOT NULL,
    sku TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0)
);
CREATE TABLE IF NOT EXISTS idempotency_requests (
    idempotency_key TEXT PRIMARY KEY,
    request_fingerprint TEXT NOT NULL,
    order_id INTEGER NOT NULL REFERENCES orders(id)
);
