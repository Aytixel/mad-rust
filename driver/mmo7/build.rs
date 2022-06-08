use std::env;
use std::fs;
use std::path::Path;

fn main() {
    copy(Path::new("icon.png"));
}

fn copy(file: &Path) {
    fs::copy(
        Path::join(&env::current_dir().unwrap(), file),
        Path::new(&env::var("OUT_DIR").unwrap()).join(Path::new("../../../").join(file)),
    )
    .unwrap();
}
