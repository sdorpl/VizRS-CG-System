//! `vizrs-model` – format show VizRS bez zależności: JSON (`json`), walidacja show przed zapisem i na antenie
//! (`validate`) oraz model tekstu tickera (`ticker`, odpowiednik `web/ticker-model.js`).
//!
//! Serwer `vizrs` jest właścicielem formatu show i korzysta z tego crate'u (plan wydzielenia Artist, A1);
//! VizRS CG Artist może sprawdzać show tą samą walidacją (zależność git z tagiem CG System).

pub mod json;
pub mod ticker;
pub mod validate;
