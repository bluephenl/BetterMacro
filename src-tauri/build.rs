fn main() {
    // Monterey is our contractual floor. Do not raise this without a fallback audit.
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.6");
    println!("cargo:rustc-link-arg=-mmacosx-version-min=12.6");
    tauri_build::build()
}
