use cxx_qt_build::CxxQtBuilder;

fn main() {
    let include_dir = std::env::var("ACLM_KWINDOWSYSTEM_INCLUDE_DIR")
        .expect("ACLM_KWINDOWSYSTEM_INCLUDE_DIR must point to KWindowSystem headers");
    let library_dir = std::env::var("ACLM_KWINDOWSYSTEM_LIBRARY_DIR")
        .expect("ACLM_KWINDOWSYSTEM_LIBRARY_DIR must point to KWindowSystem libraries");

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
