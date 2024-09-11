CREATE TYPE locale AS ENUM (
    'RU',
    'EN',
    'ZH'
);

CREATE TYPE item_status AS ENUM (
  'StatusCode'
);

CREATE TYPE currency AS ENUM (
  'USD',
  'RU'
);

CREATE TABLE IF NOT EXISTS deliveries (
  id SERIAL PRIMARY KEY,
  name VARCHAR(19),
  phone VARCHAR(11),
  zip VARCHAR(10),
  city VARCHAR(50),
  address VARCHAR(50),
  region VARCHAR(50),
  email VARCHAR(50)
);

CREATE TABLE IF NOT EXISTS payments (
  transaction VARCHAR(19) PRIMARY KEY,
  request_id VARCHAR(19),
  currency currency,
  provider VARCHAR(19),
  amount INTEGER,
  payment_dt INTEGER,
  bank VARCHAR(50),
  delivery_cost INTEGER,
  goods_total INTEGER,
  custom_fee SMALLINT
);

CREATE TABLE IF NOT EXISTS orders (
  order_uid VARCHAR(19) PRIMARY KEY,
  track_number VARCHAR(19),
  entry VARCHAR(4),
  delivery_id INTEGER REFERENCES deliveries(id),
  payment_id VARCHAR(19) REFERENCES payments(transaction),
  locale locale,
  internal_signature VARCHAR(19),
  customer_id VARCHAR(19),
  delivery_service VARCHAR(19),
  shardkey VARCHAR(2),
  sm_id INTEGER,
  date_created TIMESTAMPTZ,
  oof_shard VARCHAR(2)
);

CREATE TABLE IF NOT EXISTS items (
  id SERIAL PRIMARY KEY,
  chrt_id INTEGER,
  track_number VARCHAR(19),
  price INTEGER,
  rid VARCHAR(21),
  name VARCHAR(50),
  sale SMALLINT,
  size VARCHAR(4),
  total_price INTEGER,
  nm_id INTEGER,
  brand VARCHAR(50),
  status item_status
);

CREATE TABLE IF NOT EXISTS items_to_order (
  id SERIAL PRIMARY KEY,
  item_id INTEGER REFERENCES items(id) DEFERRABLE INITIALLY DEFERRED,
  order_id VARCHAR(19) REFERENCES orders(order_uid) DEFERRABLE INITIALLY DEFERRED
);
