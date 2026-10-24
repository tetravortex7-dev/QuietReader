fn main() {
    #[cfg(windows)]
    {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let (program, object_path, args) = if target_env == "msvc" {
            (
                "rc.exe",
                format!("{out_dir}/resources.res"),
                vec![
                    "/nologo".to_string(),
                    "/fo".to_string(),
                    format!("{out_dir}/resources.res"),
                    "resources.rc".to_string(),
                ],
            )
        } else {
            (
                "windres",
                format!("{out_dir}/resources.o"),
                vec![
                    "resources.rc".to_string(),
                    "-o".to_string(),
                    format!("{out_dir}/resources.o"),
                ],
            )
        };

        match std::process::Command::new(program).args(args).status() {
            Ok(status) if status.success() => {
                println!("cargo:rustc-link-arg={object_path}");
            }
            Ok(status) => panic!("Команда {program} завершилась с ошибкой: {status}"),
            Err(error) => {
                println!("cargo:warning=Не удалось запустить {program}: {error}");
            }
        }

        println!("cargo:rerun-if-changed=resources.rc");
        println!("cargo:rerun-if-changed=assets/app.ico");
        println!("cargo:rerun-if-changed=assets/fb2_doc.ico");
    }
}
