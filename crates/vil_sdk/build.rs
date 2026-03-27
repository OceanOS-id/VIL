// vil_sdk/build.rs
//
// ═══════════════════════════════════════════════════════════════════
// SOURCE MODE (currently active):
//   This build.rs is a no-op.
//   Cargo resolves all deps from source crates via [dependencies].
//   No static libraries need to be linked manually.
//
// IP PROTECTION MODE (placeholder — not yet active):
//   When pack_sdk.sh is active, this build.rs will:
//   1. Add link search path to sdk-dist/libs/
//   2. Link -lvil_engine (prebuilt .a)
//   3. Link -lvil_runtime (prebuilt .a)
//
//   Template for IP mode (uncomment when needed):
//
//   fn main() {
//       let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
//       let sdk_dir  = std::path::Path::new(&manifest);
//       let root_dir = sdk_dir.parent().unwrap().parent().unwrap();
//
//       // Search in sdk-dist/libs/
//       let sdk_libs = root_dir.join("sdk-dist/libs");
//       println!("cargo:rustc-link-search=native={}", sdk_libs.display());
//
//       // Link prebuilt static archives
//       println!("cargo:rustc-link-lib=static=vil_engine");
//       println!("cargo:rustc-link-lib=static=vil_runtime");
//
//       println!("cargo:rerun-if-changed=build.rs");
//   }
// ═══════════════════════════════════════════════════════════════════

fn main() {
    // SOURCE MODE: no-op
    println!("cargo:rerun-if-changed=build.rs");
}
