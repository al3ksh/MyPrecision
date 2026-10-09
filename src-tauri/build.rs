fn main() {
    // tauri-build only watches tauri.conf.json; without this a new icon never reaches the exe resources.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=app.manifest");
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri build script");
}
