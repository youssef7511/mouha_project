fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "query",
            "mutate",
            "auth",
            "workspace_backup",
            "workspace_restore",
        ]),
    ))
    .expect("Tauri build configuration failed");
}
