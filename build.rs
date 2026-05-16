fn main() {
    println!("cargo:rustc-link-search=native=./td_api/td/bin");
    println!("cargo:rustc-link-lib=dylib=tdjson");

    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../../td_api/td/bin");
}
