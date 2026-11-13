# slideshow-thing
A project for Mr. Wilson at school. All it does is read images off a USB stick and scrolls through them every 5 seconds.

### You need...
* A Debian-based computer (Debain/Ubuntu/Raspbian/etc.) A Raspberry Pi works, but it may perform poorly until I can better optimize it.
* Node.js 24 and an accompanying NPM install
* The latest stable Rust toolchain
* **yarn** installed: `npm i -g corepack`

### Setup
```bash
git clone https://github.com/WifiRouterYT/slideshow-thing # clone the repo
cd slideshow-thing
yarn # install packages
yarn run tauri build -- --no-bundle # build for your CPU architecture
# a standalone executable will be in src-tauri/target/release/

# on the machine you want to use it on, run:
sudo ./slideshow-thing setup
# this installs a script and systemd service to auto mount USB drives
```

### Usage
* Start the program after setting it up
* Format a USB stick as FAT32, and place all of your images into a folder called `img` on the root of the drive. This is where it'll search for images.
* Insert the USB stick into the computer. If all is well, it should auto mount and begin scrolling the images within a few seconds.

### Notes
* I don't think cross compiling works with some packages, so you may need to build on a Raspberry Pi if you plan on using one. A Pi 4 or better is recommended.
* The program struggles with high resolution images and will run slow w/ high memory usage. Still working on optimizing it!