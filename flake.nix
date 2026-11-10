{
  description = "A native Minecraft server management interface built with Ely";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    steel-source = {
      url = "github:Steel-Foundation/SteelMC/fb63e053ea5ecb26dc9faaa3a80e19fb20d7e079";
      flake = false;
    };
    pumpkin-source = {
      url = "github:Pumpkin-MC/Pumpkin/869f58d65059751f17744d349c700ea00166c4f2";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      steel-source,
      pumpkin-source,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib;
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        runtimeLibraries =
          with pkgs;
          lib.optionals stdenv.hostPlatform.isLinux [
            wayland
            vulkan-loader
            libGL
            libxkbcommon
            libx11
            libxi
            libxrandr
            libxcursor
            libxcb
          ];
        libPath = lib.makeLibraryPath (
          [
            pkgs.fontconfig
            pkgs.freetype
          ]
          ++ runtimeLibraries
        );

        commonArgs = {
          pname = "nylium";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          meta.description = "Native Minecraft server management with Ely";
          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
            unzip
            jq
          ];
          buildInputs =
            with pkgs;
            [
              fontconfig
              freetype
              openssl
            ]
            ++ runtimeLibraries;
        };

        # Steel's upstream pin contains a 26.2 JAR despite targeting 26.3.
        steelMetadata = builtins.fromJSON (builtins.readFile ./nix/minecraft-assets.json);
        steelAssets = pkgs.fetchurl {
          inherit (steelMetadata.serverJar) url hash;
        };
        prepareSteelBuild = ''
          # Upstream Git crates need the local core's API even in Crane's dependency build.
          cp -r ${commonArgs.src}/nylium-steel/vendor/steel-core/. nylium-steel/vendor/steel-core/
          chmod -R u+w nylium-steel/vendor/steel-core
          cp ${./nylium-steel/vendor/steel-core/build}/*.json nylium-steel/vendor/steel-core/build/
          chmod -R u+w nylium-steel/vendor/steel-core
          cp ${./nylium-steel/Cargo.lock} nylium-steel/Cargo.lock
          vendorDir="$TMPDIR/vendor"
          cp -rL --preserve=timestamps "$cargoVendorDir" "$vendorDir"
          chmod -R u+w "$vendorDir"
          if [ -f "$vendorDir/config.toml" ]; then
            sed -i "s|$cargoVendorDir|$vendorDir|g" "$vendorDir/config.toml"
          fi
          cargoVendorDir="$vendorDir"

          steelUtils=$(find "$vendorDir" -maxdepth 2 -type d -name 'steel-utils-*' -print -quit)
          test -n "$steelUtils"
          steelRoot=$(dirname "$steelUtils")
          cp -r --preserve=timestamps ${steel-source}/build ${steel-source}/package-content "$steelRoot/"
          for steelCrate in "$steelRoot"/steel-*; do
            test -f "$steelCrate/Cargo.toml" || continue
            steelCrateName=$(basename "$steelCrate")
            ln -s "$steelCrateName" "$steelRoot/''${steelCrateName%-*}"
          done
          assets="$steelUtils/build_assets"
          mkdir -p "$assets/builtin_datapacks"
          jar="$TMPDIR/server.jar"
          nestedJar=$(unzip -Z1 ${steelAssets} | grep -E '^META-INF/versions/[^/]+/[^/]+\.jar$' | head -n 1 || true)
          if [ -n "$nestedJar" ]; then
            unzip -p ${steelAssets} "$nestedJar" > "$jar"
          else
            cp ${steelAssets} "$jar"
          fi
          mkdir "$TMPDIR/server"
          unzip -q "$jar" -d "$TMPDIR/server"
          jq -e --arg version '${steelMetadata.minecraftVersion}' '.id == $version' "$TMPDIR/server/version.json" > /dev/null
          cp -r "$TMPDIR/server/data/minecraft" "$assets/builtin_datapacks/"
          cp "$TMPDIR/server/assets/minecraft/lang/en_us.json" "$assets/en_us.json"
          cp "$TMPDIR/server/assets/minecraft/lang/deprecated.json" "$assets/deprecated.json"
          cp "$TMPDIR/server/version.json" "$assets/version.json"
          printf '%s' '${steelMetadata.minecraftVersion}' > "$assets/builtin_datapacks/minecraft/.version"
        '';

        preparePumpkinBuild = ''
          cp ${./nylium-pumpkin/Cargo.lock} nylium-pumpkin/Cargo.lock
          vendorDir="$TMPDIR/pumpkin-vendor"
          cp -rL --preserve=timestamps "$cargoVendorDir" "$vendorDir"
          chmod -R u+w "$vendorDir"
          if [ -f "$vendorDir/config.toml" ]; then
            sed -i "s|$cargoVendorDir|$vendorDir|g" "$vendorDir/config.toml"
          fi
          cargoVendorDir="$vendorDir"

          # Pumpkin includes generated Rust from sibling crates by their unversioned names.
          for pumpkinCrate in "$vendorDir"/*/pumpkin-*; do
            test -f "$pumpkinCrate/Cargo.toml" || continue
            pumpkinCrateDir=$(basename "$pumpkinCrate")
            pumpkinCrateName=$(printf '%s' "$pumpkinCrateDir" | sed 's/-[0-9].*$//')
            ln -s "$pumpkinCrateDir" "$(dirname "$pumpkinCrate")/$pumpkinCrateName"
          done
          cp -r --preserve=timestamps ${pumpkin-source}/assets "$vendorDir/"
          chmod -R u+w "$vendorDir/assets"
        '';

        pumpkinArgs = commonArgs // {
          pname = "nylium-pumpkin";
          version = (builtins.fromTOML (builtins.readFile ./nylium-pumpkin/Cargo.toml)).package.version;
          cargoExtraArgs = "--manifest-path nylium-pumpkin/Cargo.toml --features ui --bin nylium-pumpkin-ui";
          cargoLock = ./nylium-pumpkin/Cargo.lock;
          PUMPKIN_MINECRAFT_SERVER_JAR = steelAssets;
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };

        pumpkinCargoArtifacts = craneLib.buildDepsOnly (
          pumpkinArgs // { preConfigure = preparePumpkinBuild; }
        );
        preparePumpkinPackage = preparePumpkinBuild;
        pumpkinServerArgs = pumpkinArgs // {
          pname = "nylium-pumpkin-server";
          cargoExtraArgs = "--manifest-path nylium-pumpkin/Cargo.toml";
          buildInputs = [ pkgs.openssl ];
        };
        pumpkinServerCargoArtifacts = craneLib.buildDepsOnly (
          pumpkinServerArgs // { preConfigure = preparePumpkinBuild; }
        );

        steelArgs = commonArgs // {
          pname = "nylium-steel";
          version = (builtins.fromTOML (builtins.readFile ./nylium-steel/Cargo.toml)).package.version;
          cargoExtraArgs = "--manifest-path nylium-steel/Cargo.toml --features ui --bin nylium-steel-ui";
          cargoLock = ./nylium-steel/Cargo.lock;
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
        steelCargoArtifacts = craneLib.buildDepsOnly (steelArgs // { preConfigure = prepareSteelBuild; });
        prepareSteelPackage = prepareSteelBuild + ''
          # Crane caches target/, but Steel generates Rust inside the fresh vendor sources.
          touch "$steelRoot"/steel-*/build/build.rs nylium-steel/vendor/steel-core/build/build.rs nylium-steel/vendor/steel-core/build/classes.json
        '';
        steelServerArgs = steelArgs // {
          pname = "nylium-steel-server";
          cargoExtraArgs = "--manifest-path nylium-steel/Cargo.toml";
          buildInputs = [ pkgs.openssl ];
        };
        steelServerCargoArtifacts = craneLib.buildDepsOnly (
          steelServerArgs // { preConfigure = prepareSteelBuild; }
        );

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        adapterInstallHooks = directory: {
          doNotPostBuildInstallCargoBinaries = true;
          postBuild = ''
            # Crane's installer resolves workspace members from its current directory.
            adapterBuildLog="$PWD/$cargoBuildLog"
            postBuildInstallFromCargoBuildLogOut=$(mktemp -d)
            (
              cd ${lib.escapeShellArg directory}
              installFromCargoBuildLog "$postBuildInstallFromCargoBuildLogOut" "$adapterBuildLog"
            )
          '';
        };

        nyliumClippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
          }
        );

        nylium = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "-p nylium";
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/nylium" --prefix LD_LIBRARY_PATH : "${libPath}"
            '';
          }
        );

        nyliumSteel = craneLib.buildPackage (
          steelArgs
          // adapterInstallHooks "nylium-steel"
          // {
            cargoArtifacts = steelCargoArtifacts;
            preConfigure = prepareSteelPackage;
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/nylium-steel-ui" \
                --prefix LD_LIBRARY_PATH : "${libPath}" \
                --set-default SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            '';
          }
        );

        nyliumPumpkin = craneLib.buildPackage (
          pumpkinArgs
          // adapterInstallHooks "nylium-pumpkin"
          // {
            cargoArtifacts = pumpkinCargoArtifacts;
            preConfigure = preparePumpkinPackage;
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/nylium-pumpkin-ui" \
                --prefix LD_LIBRARY_PATH : "${libPath}" \
                --set-default SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            '';
          }
        );
        nyliumSteelServer = craneLib.buildPackage (
          steelServerArgs
          // adapterInstallHooks "nylium-steel"
          // {
            cargoArtifacts = steelServerCargoArtifacts;
            preConfigure = prepareSteelPackage;
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/nylium-steel" \
                --set-default SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            '';
          }
        );

        nyliumPumpkinServer = craneLib.buildPackage (
          pumpkinServerArgs
          // adapterInstallHooks "nylium-pumpkin"
          // {
            cargoArtifacts = pumpkinServerCargoArtifacts;
            preConfigure = preparePumpkinPackage;
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/nylium-pumpkin" \
                --set-default SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            '';
          }
        );
      in
      {
        checks = {
          inherit
            nylium
            nyliumClippy
            ;
        };
        packages = {
          default = nylium;
          steel = nyliumSteel;
          steel-server = nyliumSteelServer;
          pumpkin = nyliumPumpkin;
          pumpkin-server = nyliumPumpkinServer;
        };
        apps.default = (flake-utils.lib.mkApp { drv = nylium; }) // {
          inherit (nylium) meta;
        };
        apps.steel =
          (flake-utils.lib.mkApp {
            drv = nyliumSteel;
            exePath = "/bin/nylium-steel-ui";
          })
          // {
            inherit (nyliumSteel) meta;
          };
        apps.steel-server =
          (flake-utils.lib.mkApp {
            drv = nyliumSteelServer;
            exePath = "/bin/nylium-steel";
          })
          // {
            inherit (nyliumSteelServer) meta;
          };
        apps.pumpkin =
          (flake-utils.lib.mkApp {
            drv = nyliumPumpkin;
            exePath = "/bin/nylium-pumpkin-ui";
          })
          // {
            inherit (nyliumPumpkin) meta;
          };
        apps.pumpkin-server =
          (flake-utils.lib.mkApp {
            drv = nyliumPumpkinServer;
            exePath = "/bin/nylium-pumpkin";
          })
          // {
            inherit (nyliumPumpkinServer) meta;
          };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [ ];
          LD_LIBRARY_PATH = libPath;
        };
      }
    );
}
