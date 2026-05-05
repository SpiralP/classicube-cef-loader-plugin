use super::*;

#[test]
fn current_lib_path_resolves_to_existing_file() {
    // In a `cargo test` run, dladdr / GetModuleHandleExW resolve to the test
    // binary itself rather than the cdylib - but it should still return some
    // valid, existing path on disk. This is a smoke test that the platform
    // FFI is wired up; the real cdylib resolution is exercised by the run.
    let path = current_lib_path().expect("current_lib_path returned an error");
    assert!(
        path.exists(),
        "current_lib_path returned non-existent path: {}",
        path.display()
    );
}
