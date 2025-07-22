# shell.nix
{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    pkg-config

    wayland-scanner # For generating Wayland protocol code

    rustfmt
    clippy
    rust-analyzer

    glfw

    mesa

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
    pkgs.libxkbcommon # <--- Explicitly add libxkbcommon here
    pkgs.libffi # Ensure libffi is in the runtime path if needed by linked libs
  ]}";

  PKG_CONFIG_PATH = "${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
    pkgs.libffi.dev # Include libffi's pkgconfig path
    pkgs.glfw # Include libffi's pkgconfig path
    pkgs.libGL.dev
    pkgs.wayland.dev # Wayland's development files include wayland-client.pc
    pkgs.libxkbcommon.dev # <--- Also ensure its .pc files are found if any are referenced
  ]}";

  RUST_BACKTRACE = 1;

  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";

  shellHook = ''
    echo "Entering glade development shell"
  '';
}
