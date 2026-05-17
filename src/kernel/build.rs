use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_script = PathBuf::from(&manifest_dir).join("kernel.ld");

    /* Tell cargo to use the linker script */
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    
    /* Tell cargo not to link with the default runtime libraries */
    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-nodefaultlibs");
    
    /* Position-independent code */
    println!("cargo:rustc-link-arg=-fno-pie");
}
