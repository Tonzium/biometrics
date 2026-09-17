-- Omistajan poistamat harjoitukset (poistolista).
--
-- Synkronointi on pelkkää upsertia eikä koskaan poista rivejä, joten Polar
-- Flow'sta poistettu harjoitus jäisi kantaan pysyvästi. Omistaja voi poistaa
-- harjoituksen käyttöliittymästä (DELETE /api/exercises/{id}). Pelkkä DELETE
-- ei kuitenkaan riitä: Polarin harjoituslista kattaa 30 päivää, ja jos Polar
-- palauttaa saman id:n uudelleen, upsert toisi rivin takaisin.
--
-- Siksi poisto kirjaa id:n tähän tauluun samassa transaktiossa, ja
-- `upsert_exercise` ohittaa id:t, jotka löytyvät täältä. Lukureitit ja
-- näkymät eivät muutu, koska poistettu rivi on aidosti poissa `exercises`-taulusta.
-- Palautus onnistuu poistamalla rivi täältä ja ajamalla synkronointi.
CREATE TABLE deleted_exercises (
    id               text        PRIMARY KEY,                 -- Polarin hash-id
    polar_account_id uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    deleted_at       timestamptz NOT NULL DEFAULT now()
);
