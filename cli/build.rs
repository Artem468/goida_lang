fn main() {
    println!("cargo:rerun-if-changed=../assets/icon.ico");

    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("../assets/icon.ico")
        .compile()
        .expect("failed to embed the application icon");
}
