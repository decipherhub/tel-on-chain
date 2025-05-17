fn main() {
    // Ensure libraries are linked correctly
    println!("cargo:rustc-link-lib=sqlite3");

    // Rerun this script if the build configuration changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
