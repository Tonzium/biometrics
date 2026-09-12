-- Istuntojen mitätöinti palvelimen puolelta.
--
-- Istunto on tilaton JWT, joten palvelin ei voi "poistaa" vielä voimassa
-- olevaa tokenia. Ainoa keino oli tähän asti vaihtaa JWT_SECRET, mikä
-- mitätöi kaikkien käyttäjien istunnot ja vaatii uudelleenkäynnistyksen.
--
-- Istuntoversio ratkaisee sen käyttäjäkohtaisesti: sama luku kulkee tokenin
-- `ver`-kentässä, ja jokainen pyyntö vertaa sitä kannan arvoon.
-- "Kirjaa minut ulos kaikkialta":
--   UPDATE app_users SET token_version = token_version + 1 WHERE email = '...';
-- Uusi kirjautuminen toimii heti, koska se saa tokeniin uuden version.
--
-- Luku eikä aikaleima, koska JWT:n `iat` on sekunnin tarkkuudella: aikaleimaan
-- vertaaminen olisi epämääräinen kuluvan sekunnin sisällä ja hylkäisi herkästi
-- juuri myönnetyn tokenin.

ALTER TABLE app_users
    ADD COLUMN token_version integer NOT NULL DEFAULT 1;
