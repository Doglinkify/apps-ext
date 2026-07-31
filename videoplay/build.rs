// build.rs — emit explicit linker search paths and dynamic lib links for SDL2.
//
// On the dlos target we don't link anything — the kernel loads us — so
// this script is a no-op when `--features dlos` is on.

fn main() {
    #[cfg(feature = "dlos")]
    {
        return;
    }

    #[cfg(not(feature = "dlos"))]
    {
        if let Ok(prefix) = std::env::var("SDL2_PREFIX") {
            let lib = format!("{prefix}/lib");
            let lib64 = format!("{prefix}/lib/x86_64-linux-gnu");
            println!("cargo:rustc-link-search=native={lib}");
            println!("cargo:rustc-link-search=native={lib64}");
        }
        println!("cargo:rustc-link-lib=dylib=SDL2");
    }
}
