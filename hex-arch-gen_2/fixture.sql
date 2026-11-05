-- Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
-- Author: Awase Khirni Syed
--
-- fixture.sql — demo PostgreSQL schema for hex-gen.
--
-- Loads the same schema that `hex-gen --fixture` uses internally, so you
-- can exercise the LIVE introspection path end-to-end:
--
--   psql -h localhost -U <user> -d <database> -f fixture.sql
--   hex-gen --database-url "postgresql://<user>:<pass>@localhost:5432/<database>" ./generated-live
--
-- The tables deliberately exercise every feature detector:
--   users    : native PG enum, jsonb, text[], timestamps, *_status column
--   orders   : FK → users(id), numeric money, soft delete, user-ref, timestamps
--   product_categories : integer serial PK, varchar status, keyword column (`type`)

-- ── native enum types ────────────────────────────────────────────────────────
CREATE TYPE user_status  AS ENUM ('active', 'inactive', 'suspended');
CREATE TYPE order_status AS ENUM ('pending', 'paid', 'shipped', 'delivered', 'cancelled');

-- ── users ────────────────────────────────────────────────────────────────────
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       VARCHAR(320) NOT NULL,
    full_name   VARCHAR(200),
    user_status user_status  NOT NULL DEFAULT 'active',
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    metadata    JSONB,                       -- exercises serde_json::Value
    tags        TEXT[],                      -- exercises Vec<String>
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- ── orders ───────────────────────────────────────────────────────────────────
CREATE TABLE orders (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID         NOT NULL REFERENCES users (id),
    amount       NUMERIC(12,2) NOT NULL,      -- exercises rust_decimal::Decimal
    order_status order_status NOT NULL DEFAULT 'pending',
    notes        TEXT,
    is_deleted   BOOLEAN      NOT NULL DEFAULT FALSE,
    deleted_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- ── product_categories ───────────────────────────────────────────────────────
CREATE TABLE product_categories (
    id         SERIAL PRIMARY KEY,
    name       VARCHAR(200) NOT NULL,
    type       VARCHAR(64)  NOT NULL,        -- exercises r#type keyword handling
    status     VARCHAR(32)  NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- ── sample rows ──────────────────────────────────────────────────────────────
INSERT INTO users (email, full_name, metadata, tags) VALUES
    ('awase@example.com', 'Awase Khirni Syed',
     '{"company": "ORI Inc.", "country": "Canada"}', ARRAY['founder', 'architect']),
    ('ada@example.com', 'Ada Lovelace',
     '{"team": "platform"}', ARRAY['engineer']);

INSERT INTO orders (user_id, amount, order_status, notes)
SELECT id, 149.99, 'paid', 'First order'
FROM users WHERE email = 'awase@example.com';

INSERT INTO product_categories (name, type) VALUES
    ('Books', 'physical'),
    ('E-books', 'digital');
