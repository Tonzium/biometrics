-- Sovelluksen käyttäjät ja niihin liitetyt Polar-tilit.

CREATE TABLE app_users (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text        NOT NULL UNIQUE CHECK (email = lower(email) AND email <> ''),
    password_hash text        NOT NULL,                       -- argon2id PHC-merkkijono
    role          text        NOT NULL DEFAULT 'owner' CHECK (role IN ('owner', 'viewer')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER app_users_set_updated_at
    BEFORE UPDATE ON app_users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Yksi Polar-tili per sovelluskäyttäjä. Access token salataan levossa
-- (AES-256-GCM, tavut = nonce || ciphertext), avain APP_ENCRYPTION_KEY.
CREATE TABLE polar_accounts (
    id               uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    app_user_id      uuid        NOT NULL UNIQUE REFERENCES app_users(id) ON DELETE CASCADE,
    polar_user_id    bigint      NOT NULL UNIQUE,              -- x_user_id token-vastauksesta
    access_token_enc bytea       NOT NULL,
    member_id        text        NOT NULL,                     -- oma tunniste POST /v3/users -rekisteröinnissä
    registered_at    timestamptz NOT NULL DEFAULT now(),
    last_sync_at     timestamptz,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER polar_accounts_set_updated_at
    BEFORE UPDATE ON polar_accounts
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
