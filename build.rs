use cxx_qt_build::CxxQtBuilder;

fn main() {
    build_controller_helper();
    let include_dir = std::env::var("KORSA_KWINDOWSYSTEM_INCLUDE_DIR")
        .expect("KORSA_KWINDOWSYSTEM_INCLUDE_DIR must point to KWindowSystem headers");
    let library_dir = std::env::var("KORSA_KWINDOWSYSTEM_LIBRARY_DIR")
        .expect("KORSA_KWINDOWSYSTEM_LIBRARY_DIR must point to KWindowSystem libraries");

    unsafe {
        CxxQtBuilder::new()
            .qt_module("Network")
            .qt_module("Widgets")
            .qrc("assets/resources.qrc")
            .cpp_file("src/window_effects.cpp")
            .cpp_file("src/widgets.cpp")
            .cc_builder(|builder| {
                builder.include(&include_dir);
                builder.include(std::path::Path::new(&include_dir).join("KWindowSystem"));
            })
            .files(["src/backend.rs"])
            .build();
    }

    println!("cargo:rustc-link-search=native={library_dir}");
    println!("cargo:rustc-link-lib=dylib=KF6WindowSystem");
}

fn build_controller_helper() {
    println!("cargo:rerun-if-changed=src/controller_helper.c");
    println!("cargo:rerun-if-env-changed=KORSA_MINGW_CC");
    let compiler =
        std::env::var_os("KORSA_MINGW_CC").unwrap_or_else(|| "x86_64-w64-mingw32-gcc".into());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
        .join("korsa-controller-helper.exe");
    let status = std::process::Command::new(compiler)
        .args([
            "-std=c11",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-municode",
            "-mwindows",
            "-static",
        ])
        .arg("src/controller_helper.c")
        .arg("-o")
        .arg(output)
        .args(["-ldinput8", "-ldxguid", "-lole32", "-lshell32"])
        .status()
        .expect("install the MinGW-w64 x86_64 C compiler or set KORSA_MINGW_CC");
    assert!(
        status.success(),
        "could not build the Wine controller helper"
    );
}
