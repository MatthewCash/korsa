{
  description = "Korsa development environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          qt = pkgs.symlinkJoin {
            name = "korsa-qt";
            paths = with pkgs.kdePackages; [
              kirigami
              kwindowsystem
              qqc2-desktop-style
              qtstyleplugin-kvantum
              qtbase
              qtbase.dev
              qtdeclarative
              qtdeclarative.dev
            ];
          };
          qmake = pkgs.writeShellScript "korsa-qmake" ''
            ${pkgs.kdePackages.qtbase}/bin/qmake "$@" \
              | ${pkgs.gnused}/bin/sed \
                  -e 's|${pkgs.kdePackages.qtbase.dev}|${qt}|g' \
                  -e 's|${pkgs.kdePackages.qtbase}|${qt}|g'
          '';
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "korsa";
            version = "0.1.0";
            src = pkgs.lib.cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              clang
              pkg-config
              kdePackages.wrapQtAppsHook
            ];
            buildInputs = with pkgs; [
              kdePackages.plasma-integration
              kdePackages.kwindowsystem
              qt
            ];

            QMAKE = qmake;
            KORSA_KWINDOWSYSTEM_INCLUDE_DIR = "${pkgs.kdePackages.kwindowsystem.dev}/include/KF6";
            KORSA_KWINDOWSYSTEM_LIBRARY_DIR = "${pkgs.kdePackages.kwindowsystem}/lib";

            preBuild = ''
              export QMAKE=${qmake}
            '';
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          qt = pkgs.symlinkJoin {
            name = "korsa-qt";
            paths = with pkgs.kdePackages; [
              kirigami
              kwindowsystem
              qqc2-desktop-style
              qtstyleplugin-kvantum
              qtbase
              qtbase.dev
              qtdeclarative
              qtdeclarative.dev
            ];
          };
          qmake = pkgs.writeShellScript "korsa-qmake" ''
            ${pkgs.kdePackages.qtbase}/bin/qmake "$@" \
              | ${pkgs.gnused}/bin/sed \
                  -e 's|${pkgs.kdePackages.qtbase.dev}|${qt}|g' \
                  -e 's|${pkgs.kdePackages.qtbase}|${qt}|g'
          '';
        in {
          default = pkgs.mkShell {
            hardeningDisable = [ "fortify" ];

            packages = with pkgs; [
              cargo
              clang
              clippy
              cmake
              kdePackages.qtbase
              kdePackages.qtdeclarative
              lld
              ninja
              pkg-config
              rustc
              rustfmt
            ];

            RUSTFLAGS = "-C link-arg=-fuse-ld=lld -C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath [ qt pkgs.kdePackages.kwindowsystem pkgs.stdenv.cc.cc.lib ]}";
            KORSA_KWINDOWSYSTEM_INCLUDE_DIR = "${pkgs.kdePackages.kwindowsystem.dev}/include/KF6";
            KORSA_KWINDOWSYSTEM_LIBRARY_DIR = "${pkgs.kdePackages.kwindowsystem}/lib";

            shellHook = ''
              export QMAKE=${qmake}
              export QT_PLUGIN_PATH=${qt}/lib/qt-6/plugins''${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}
              export QML2_IMPORT_PATH=${qt}/lib/qt-6/qml
              export QT_QUICK_CONTROLS_STYLE=org.kde.desktop
            '';
          };
        });
    };
}
