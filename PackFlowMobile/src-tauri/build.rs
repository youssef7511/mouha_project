fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "mobile_query",
            "mobile_mutate",
            "mobile_transfer",
            "mobile_backup",
            "mobile_restore",
        ]),
    ))
    .expect("Mobile build configuration failed");
}
