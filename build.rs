// Иконка и сведения о версии для Windows-сборки (MinGW): готовый COFF-объект из installer/.
fn main() {
    println!("cargo:rerun-if-changed=installer/carbang_res.o");
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") && target.contains("gnu") {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let res = format!("{}/installer/carbang_res.o", dir);
        if std::path::Path::new(&res).exists() {
            println!("cargo:rustc-link-arg-bins={}", res);
        }
    }
}
