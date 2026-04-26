-- Create refresh_tokens table
-- Maps from MongoDB "refreshtokens" collection (Mongoose schema)
-- Original fields: token, userId (ObjectId ref User), userEmail (ref User), expires

CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_email VARCHAR(255) NOT NULL,
    expires TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on token (matches MongoDB index: true)
CREATE INDEX idx_refresh_tokens_token ON refresh_tokens (token);

-- Index on user_email for lookup during refresh
CREATE INDEX idx_refresh_tokens_user_email ON refresh_tokens (user_email);

-- Index on user_id for cascade and lookups
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens (user_id);
