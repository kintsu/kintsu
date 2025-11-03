fn main() {
    #[cfg(feature = "fs-test")]
    {
        println!("cargo:rustc-cfg=fs_test");
    }

    println!("cargo::rustc-check-cfg=cfg(fs_test)");
}
