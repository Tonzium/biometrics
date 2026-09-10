//! Tietokantakerros. Jokainen alimoduuli vastaa yhtä taulua tai
//! aihealuetta ja käyttää `sqlx::query!`-makroja, jotka tarkistetaan
//! käännösaikana kantaa (tai `.sqlx`-offline-dataa) vasten.

pub mod polar_accounts;
pub mod polar_data;
pub mod sync_runs;
pub mod users;
