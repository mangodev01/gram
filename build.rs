fn main() {
    println!("cargo:rustc-link-search=native=./td_api/td/bin");
    println!("cargo:rustc-link-lib=dylib=tdjson");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../../td_api/td/bin");
    } else {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../../td_api/td/bin");
    }
}
