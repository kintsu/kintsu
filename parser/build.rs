fn main() {
    #[cfg(feature = "api")]
    {
        println!("cargo:rustc-cfg=api");
    }

    println!("cargo:rustc-check-cfg=cfg(api)");
}
