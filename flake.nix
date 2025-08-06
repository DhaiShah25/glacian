{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustc
        cargo
        pkg-config
        renderdoc

        rustfmt
        clippy
        rust-analyzer
        wayland-scanner

        glfw

        vulkan-validation-layers

        wayland
        wayland-protocols
        libxkbcommon
        glfw
        libGL
        cmake
        extra-cmake-modules
        vulkan-loader
        libxkbcommon
        libffi
        egl-wayland
        shader-slang
        vulkan-tools-lunarg
      ];

      LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [
        pkgs.wayland
        pkgs.vulkan-loader
        pkgs.egl-wayland
        pkgs.libGL
        pkgs.libxkbcommon
        pkgs.libffi
      ]}";

      PKG_CONFIG_PATH = "${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
        pkgs.libffi.dev
        pkgs.glfw
        pkgs.libGL.dev
        pkgs.wayland.dev
        pkgs.libxkbcommon.dev
      ]}";

      VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";

      shellHook = ''
        echo "Entering glade development shell"
        exec nu
      '';
    };
  };
}
