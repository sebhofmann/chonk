//! Bettet unter Windows das App-Icon in die `chonk.exe` ein.

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/chonk.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/icons/chonk.ico")
        .compile()
        .expect("could not embed the icon");
}
