{
  description = "A basic flake providing a devShell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { nixpkgs, ... }: {
    devShells = builtins.mapAttrs (system: pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          # replace by the packages you need
          openssl
        ];
        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
      };
    }) nixpkgs.legacyPackages;
  };
}
