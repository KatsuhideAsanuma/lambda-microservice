fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("K8S_OPENAPI_ENABLED_VERSION").is_err() {
        std::env::set_var("K8S_OPENAPI_ENABLED_VERSION", "1.26");
    }
    
    Ok(tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["src/proto/runtime.proto"], &["src"])?)
}
