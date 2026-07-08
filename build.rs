use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_dir = lib_dir(&manifest_dir);
    let lib_dir = lib_dir.canonicalize().unwrap_or(lib_dir);
    println!("cargo:rerun-if-env-changed=ROCHEDB_LIB_DIR");
    println!("cargo:rerun-if-env-changed=ROCHEDB_CORE_DIR");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=rochedb");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}

fn lib_dir(manifest_dir: &Path) -> PathBuf {
    if let Ok(dir) = std::env::var("ROCHEDB_LIB_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("ROCHEDB_CORE_DIR") {
        return PathBuf::from(dir).join("lib");
    }

    let candidates = [
        manifest_dir.join("../rochedb/lib"),
        manifest_dir.join("../ceresdb/lib"),
        manifest_dir.join("../../lib"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join(shared_lib_name()).exists())
        .unwrap_or_else(|| manifest_dir.join("../rochedb/lib"))
}

fn shared_lib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "librochedb.dylib"
    } else {
        "librochedb.so"
    }
}
