//! Sovelluksen yhteiset tietomallit ja virhetyypit.
//!
//! Tämä crate ei tiedä mitään HTTP:stä eikä tietokannasta; se määrittelee
//! vain rakenteet, joita `api` ja `polar-client` jakavat.

pub mod error;

pub use error::DomainError;
