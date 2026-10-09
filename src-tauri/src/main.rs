// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, out, ..] = args.as_slice()
        && flag == "--probe"
    {
        if let Err(e) = myprecision_lib::probe::run_probe(std::path::Path::new(out)) {
            eprintln!("probe failed: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    myprecision_lib::run();
}
