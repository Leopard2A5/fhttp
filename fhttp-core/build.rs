use std::env;
use std::path::PathBuf;

use deno_core::{extension, JsRuntimeForSnapshot, RuntimeOptions};

fn main() {
    extension!(fhttp, js = ["src/postprocessing/bootstrap.js"]);

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = out_dir.join("FHTTP_SNAPSHOT.bin");
    let options = RuntimeOptions {
        extensions: vec![fhttp::init_ops_and_esm()],
        ..Default::default()
    };
    let isolate = JsRuntimeForSnapshot::new(options);

    let snapshot = isolate.snapshot();
    let snapshot_slice: &[u8] = &snapshot;
    println!("Snapshot size: {}", snapshot_slice.len());
    std::fs::write(&snapshot_path, snapshot_slice).unwrap();
    println!("Snapshot written to: {} ", snapshot_path.display());
}
