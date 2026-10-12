fn main() {
    #[cfg(windows)]
    {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let obj_path = format!("{out_dir}/resources.o");

        let windres_cmd = if std::process::Command::new("windres")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "windres".to_string()
        } else {
            "C:\\Users\\Almas\\AppData\\Local\\Microsoft\\WinGet\\Packages\\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\\mingw64\\bin\\windres.exe".to_string()
        };

        let status = std::process::Command::new(&windres_cmd)
            .args(["resources.rc", "-o", &obj_path])
            .status()
            .unwrap_or_else(|e| panic!("Не удалось запустить {windres_cmd}: {e}"));

        assert!(status.success(), "Команда windres завершилась с ошибкой");

        println!("cargo:rustc-link-arg={obj_path}");
        println!("cargo:rerun-if-changed=resources.rc");
        println!("cargo:rerun-if-changed=assets/app.ico");
        println!("cargo:rerun-if-changed=assets/fb2_doc.ico");
    }
}
