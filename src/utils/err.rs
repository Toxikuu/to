// utils/err.rs

pub mod eyre_prelude {
    pub use color_eyre::eyre::{Report as Ereport, Result as Eresult, WrapErr};
}
