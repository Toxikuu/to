// utils/debug.rs
//! Debugging utilities
//! Everything defined here is only active whenever `debug_assertions` are enabled

#[cfg(debug_assertions)]
pub fn __dbug<D: std::fmt::Debug>(target: D) {
    eprintln!("\x1b[38;1m DBUG\x1b[39m :::\x1b[0m {target:#?}");
}

#[cfg(debug_assertions)]
pub fn __unravel(e: &impl std::error::Error) {
    tracing::error!("Error: {e}");
    let mut source = e.source();
    while let Some(e) = source {
        tracing::error!("    Caused by: {e}");
        source = e.source();
    }
}

/// # Unravels a chain of errors as long as sources exist
#[macro_export]
macro_rules! unravel {
    ($e: expr) => {
        #[cfg(debug_assertions)]
        {
            $crate::utils::debug::__unravel(&$e);
        }
    };
}

/// # Prints a debugging message
#[macro_export]
macro_rules! dbug {
    ($e: expr) => {
        #[cfg(debug_assertions)]
        {
            $crate::utils::debug::__dbug(&$e);
        }
    };
}
