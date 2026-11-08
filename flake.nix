{
  description = "A basic flake providing a devShell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { nixpkgs, ... }: {
    devShells = builtins.mapAttrs (system: pkgs: {
      default = pkgs.mkShell {
        buildInputs = with pkgs; [
          fish
        ];
        packages = with pkgs; [
          # replace by the packages you need
          openssl
          python3
        ];

        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
        shellHook = ''
        if [[ $- == *i* ]]; then
          exec ${pkgs.fish}/bin/fish --init-command '
          function fish_prompt
            set git (fish_git_prompt)
            set -l branch (git branch --show-current 2>/dev/null)
            echo -n (set_color red)"devshell"(set_color normal)
            echo -n "|"(set_color green)(prompt_pwd)(set_color normal)" "

            if test -n "$branch"
              if test "$branch" = "main"
                echo -n "("(set_color red)"$branch"(set_color normal)")"
              else
                echo -n "("(set_color green)"$branch"(set_color normal)")"
              end
            end
            #echo -n $git
            echo -n "> "
          end
          '
        fi
        '';
      };
    }) nixpkgs.legacyPackages;
  };
}
