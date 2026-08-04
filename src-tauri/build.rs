fn main() {
    #[cfg(target_os = "macos")]
    {
        let development_driver = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/oracle/instantclient_19_16");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            development_driver.display()
        );
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Resources/oracle/instantclient_19_16"
        );
    }
    tauri_build::build()
}
