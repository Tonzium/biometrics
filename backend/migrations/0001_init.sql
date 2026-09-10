-- Alustusmigraatio: laajennukset ja apufunktio päivitysleimoille.
-- Varsinaiset taulut tulevat seuraavissa migraatioissa (vaihe 1-3).

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;
