# All cargoes that the project runs are:

 - bitflags: a crate for defining bitflag types
 - cfg-if: a small macro crate for defining cfg-based  -   conditional compilation
 - lazy_static: a crate for defining lazy-initialized static variables
 - libc: a crate for defining the C library interface in Rust
 - sdl2: a Rust wrapper around the SDL2 library for graphics and input handling
 - sdl2-sys: a low-level Rust binding to the SDL2 library
 - software-renderer: a package that depends on sdl2 and implements a software renderer
 - version-compare: a crate for comparing version strings
 
## Screenshots

![Cube](https://cdn.discordapp.com/attachments/1045463014876397638/1100865694469468160/image_1.png)

![Cube with a frame](https://cdn.discordapp.com/attachments/1045463014876397638/1100865705366261790/image.png)


## Run Locally

Clone the project

```bash
  git clone https://github.com/Valery-a/Rust-renderer.git
```

Go to the project directory

```bash
  cd Rust-renderer
```

Build the project

```bash
  cargo build
```

Start the app

```bash
  cargo run --release
```


## Error handling

#### If you are experiencing any errors such has this one:
- error: linking with `link.exe` failed: exit code: 1181

Follow these steps to resolve it:
```
Download MSVC development libraries from http://www.libsdl.org/ (SDL2-devel-2.0.x-VC.zip).

Unpack SDL2-devel-2.0.x-VC.zip to a folder of your choosing (You can delete it afterwards).

Copy all lib files from

SDL2-devel-2.0.x-VC\SDL2-2.0.x\lib\x64\

to (for Rust 1.6 and above)

C:\Program Files\Rust\lib\rustlib\x86_64-pc-windows-msvc\lib

or to (for Rust versions 1.5 and below)

C:\Program Files\Rust\bin\rustlib\x86_64-pc-windows-msvc\lib

or to your library folder of choice, and ensure you have a system environment variable of

LIB = C:\your\rust\library\folder

For Rustup users, this folder will be in

C:\Users\{Your Username}\.rustup\toolchains\{current toolchain}\lib\rustlib\{current toolchain}\lib

Where current toolchain is likely stable-x86_64-pc-windows-msvc.

Copy SDL2.dll from

SDL2-devel-2.0.x-VC\SDL2-2.0.x\lib\x64\

into your cargo project, right next to your Cargo.toml.

When you're shipping your game make sure to copy SDL2.dll to the same directory that your compiled exe is in, otherwise the game won't launch.
```
