{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };
  outputs = flakes: flakes.flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import flakes.nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };
      craneLib = flakes.crane.mkLib pkgs;
      dioxus-cli = craneLib.buildPackage {
        src = craneLib.downloadCargoPackage {
          name = "dioxus-cli";
          version = "0.5.7";
          checksum = "sha256-Xjn9+/evKma26oJwzTaE6gF60MQP8q9fPTWnL7Oz/Uw=";
          source = "registry+https://github.com/rust-lang/crates.io-index";
        };
        buildInputs = [
          pkgs.pkg-config
          pkgs.openssl
        ];
        doCheck = false;
        OPENSSL_NO_VENDOR = 1;
      };
    in
    {
      devShell = pkgs.mkShell {
        buildInputs = [
        ];
        packages = [
          pkgs.rustup
          pkgs.nodejs
          dioxus-cli
          pkgs.pkg-config
          pkgs.openssl
        ];
      };
    }
  );
}

