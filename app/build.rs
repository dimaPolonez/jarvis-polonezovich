fn main() {
    // link to Vosk lib
    println!("cargo:rustc-link-search=native=.");
    println!("cargo:rustc-link-lib=dylib=vosk");
}
