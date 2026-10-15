// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().nth(1).as_deref() == Some("mcp") {
        if let Err(error) = spicedepot_lib::run_mcp() {
            eprintln!("SpiceDepot MCP server: {error}");
            std::process::exit(1);
        }
        return;
    }
    spicedepot_lib::run();
}
