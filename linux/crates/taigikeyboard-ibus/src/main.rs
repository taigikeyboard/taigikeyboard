//! `ibus-engine-taigikeyboard`: the process the daemon spawns from the
//! component XML (`<exec>… --ibus</exec>`). It joins the daemon's bus,
//! takes the well-known name, serves the factory, and lives until the bus
//! goes away — the daemon owns its lifetime.
//!
//! Design record: `docs/architecture/linux-roadmap.md` (L1, L13).

mod bus;
mod engine;
mod factory;
mod wire;

use std::process::ExitCode;
use std::sync::Arc;

/// The well-known bus name the component XML declares.
const BUS_NAME: &str = "org.freedesktop.IBus.TaigiKeyboard";

fn main() -> ExitCode {
    taigi_linux_platform::install_debug_logger();
    // `--ibus` is the daemon's convention for "started by ibus-daemon";
    // there is no other way to run this binary, so the flag is accepted and
    // nothing else is parsed.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let address = match bus::address() {
        Ok(address) => address,
        Err(error) => {
            log::error!("bus.no_address error={error}");
            eprintln!("ibus-engine-taigikeyboard: {error}");
            return ExitCode::FAILURE;
        }
    };
    log::info!(
        "engine.start version={} address={address}",
        env!("CARGO_PKG_VERSION")
    );
    match zbus::block_on(serve(&address)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("engine.stopped error={error}");
            eprintln!("ibus-engine-taigikeyboard: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(address: &str) -> zbus::Result<()> {
    let runtime = Arc::new(taigi_linux_core::Runtime::probe());
    let connection = zbus::connection::Builder::address(address)?
        .serve_at(factory::FACTORY_PATH, factory::Factory::new(runtime))?
        .name(BUS_NAME)?
        .build()
        .await?;
    log::info!(
        "bus.connected unique_name={:?}",
        connection
            .unique_name()
            .map(|name| name.as_str().to_owned())
    );
    engine::watch_popup(connection.clone());
    // Until the daemon drops the connection.
    connection.closed().await;
    log::info!("bus.closed");
    Ok(())
}
