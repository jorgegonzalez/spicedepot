use std::path::{Path, PathBuf};

fn main() {
    // Compile authzed gRPC protos via tonic-build. We only need the client side.
    // Proto files are vendored under src-tauri/proto/ — see scripts/sync-proto.sh.
    let proto_root: PathBuf = Path::new("proto").to_path_buf();

    let services = [
        "authzed/api/v1/permission_service.proto",
        "authzed/api/v1/schema_service.proto",
        "authzed/api/v1/experimental_service.proto",
        "authzed/api/v1/watch_service.proto",
        // google.rpc.Status is referenced from authzed v1 oneofs; compiling it
        // here gives us a real `google::rpc` module to mount in proto.rs.
        "google/rpc/status.proto",
    ];
    let service_paths: Vec<PathBuf> =
        services.iter().map(|p| proto_root.join(p)).collect();

    // Re-run if any proto file changes.
    println!("cargo:rerun-if-changed=proto");
    for p in &service_paths {
        println!("cargo:rerun-if-changed={}", p.display());
    }

    // Use a vendored protoc so contributors don't have to install one. Honor
    // an externally-set $PROTOC if the user wants their own version.
    if std::env::var_os("PROTOC").is_none() {
        let protoc = protoc_bin_vendored::protoc_bin_path()
            .expect("vendored protoc not available for this platform");
        std::env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&service_paths, &[proto_root])
        .expect("failed to compile authzed protos");

    tauri_build::build();
}
