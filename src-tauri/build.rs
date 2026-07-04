fn main() {
    // tauri_build::build() may panic in sandboxed environments where
    // CreateProcessW is blocked for the rustc subprocess spawned by
    // embed_resource::compile -> rustc_version::version. All cfg/env
    // directives are emitted before the panic, so we catch it and
    // continue without Windows resource (icon/manifest) embedding.
    // TODO(tech-debt): remove catch_unwind once sandbox no longer blocks rustc subprocess
    let result = std::panic::catch_unwind(|| {
        tauri_build::build()
    });
    if result.is_err() {
        eprintln!("warning: tauri_build::build() panicked (likely sandbox blocking rustc subprocess); continuing without Windows resource embedding");
    }
}
