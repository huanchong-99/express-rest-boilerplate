-- Create users table
-- Maps from MongoDB "users" collection (Mongoose schema)
-- Original fields: email, password, name, services (facebook, google), role, picture, createdAt, updatedAt

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    password VARCHAR(128) NOT NULL,
    name VARCHAR(128),
    role VARCHAR(16) NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    picture TEXT,
    facebook_id VARCHAR(255),
    google_id VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique index on email (matches MongoDB unique: true)
CREATE UNIQUE INDEX idx_users_email ON users (LOWER(email));

-- Index on name (matches MongoDB index: true)
CREATE INDEX idx_users_name ON users (name);

-- Index on created_at for sorting (matches MongoDB sort: { createdAt: -1 })
CREATE INDEX idx_users_created_at ON users (created_at DESC);

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
