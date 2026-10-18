fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/app-icon.ico");
        resource.set("ProductName", "RShell");
        resource.set("FileDescription", "RShell Remote Workbench");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
