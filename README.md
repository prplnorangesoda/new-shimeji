# Building

Arch Linux cross compilation for Windows:

```
# pacman -S mingw-w64

$ rustup target add x86_64-pc-windows-gnu

$ PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-w64-mingw32/ cargo build --target x86_64-pc-windows-gnu
```
