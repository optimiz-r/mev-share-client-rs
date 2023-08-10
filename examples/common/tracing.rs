use std::{env::VarError, error::Error};
use tracing_subscriber::EnvFilter;

/// Subscribes to tracing events that will output to the console.
///
/// By default, it enables:
///
/// * `warn` level (and higher) on all modules,
/// * `debug` level (and higher) on the `mev_share_rs` module,
/// * all levels on the current module.
///
/// You can override these defaults by setting the `RUST_LOG` env variable, e.g.:
///
/// ```sh
/// $ RUST_LOG=info cargo run --example send_private_tx
/// ```
pub fn init_tracing() {
    _init_tracing(None)
}

pub fn init_tracing_with_level(level: &str) {
    _init_tracing(Some(level))
}

// Internal API //

fn _init_tracing(level: Option<&str>) {
    let default_env_filter = format!(
        "warn,mev_share_rs=debug,{}{}", 
        current_exe(), 
        level.map(|lvl| format!("={lvl}")).unwrap_or_default()
    );

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| 
        if let Some(source) = err.source() && let Some(VarError::NotPresent) = source.downcast_ref::<VarError>() {
            EnvFilter::try_new(default_env_filter).unwrap()
        } else {
            panic!("{err:?}")
        });

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}

fn current_exe() -> String {
    std::env::current_exe()
        .expect("Failed to get current executable")
        .file_stem()
        .expect("Failed to get file stem")
        .to_str()
        .expect("Failed to convert file stem to &str")
        .to_string()
}
