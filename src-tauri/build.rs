fn main() {
    // O crate `screencapturekit` linka o runtime Swift (libswift_Concurrency.dylib
    // e afins). Sem um rpath para as bibliotecas Swift do sistema, o dyld não acha
    // essas dylibs e o app morre no launch ("Library not loaded: @rpath/...").
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/System/Library/Frameworks");
    }
    tauri_build::build()
}
