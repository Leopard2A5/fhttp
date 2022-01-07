use std::env;
use std::path::PathBuf;

use deno_core::{JsRuntime, RuntimeOptions};

fn main() {
    let fhttp_extension = deno_core::Extension::builder()
        .js(deno_core::include_js_files!(
            prefix "fhttp",
            "src/postprocessing/bootstrap.js",
        ))
        .build();

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = out_dir.join("FHTTP_SNAPSHOT.bin");
    let options = RuntimeOptions {
        will_snapshot: true,
        extensions: vec![
            fhttp_extension,
        ],
        ..Default::default()
    };
    let mut isolate = JsRuntime::new(options);

    let snapshot = isolate.snapshot();
    let snapshot_slice: &[u8] = &*snapshot;
    println!("Snapshot size: {}", snapshot_slice.len());
    std::fs::write(&snapshot_path, snapshot_slice).unwrap();
    println!("Snapshot written to: {} ", snapshot_path.display());
}
