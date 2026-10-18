fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "query",
            "mutate",
            "auth",
            "workspace_backup",
            "workspace_restore",
            "workspace_export",
        ]),
    ))
    .expect("Tauri build configuration failed");
}
