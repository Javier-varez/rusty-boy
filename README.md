# Rusty Boy

`Rusty Boy` is an open-source `Game Boy` emulator written in `Rust` for `Linux`, `macOS` and [`Playdate`](https://play.date).
`Rusty Boy` does not aim to be the most accurate `Game Boy` emulator, or the most feature complete. Instead,
it is a simplistic implementation of the minimum set of features required to play most games. It focuses
on performance on `Playdate`, often at the cost of emulation accuracy.

## Motivation

I started this project because, even though several other emulators had been written and ported to the
`Playdate` handheld console, I found they ran staggeringly slow, to the point that they were unplayable on
a real device, specially in revision B of the `Playdate`.

## Features

- Emulates most of the `DMG` `Game Boy` features.
- Achieves 70 % to 100 % of the framerate in the `Playdate`, depending on the game and the kind of load it requires.
- Upscales the frame to the size of the `Playdate` screen, and applies dithering to emulate the gray shades of the
original `DMG` Game Boy.
- Emulates `MBC1`, `rom-only` and `MBC3` cartridges. Adding support for other mappers should be easy to do.
- Backs up cartridge RAM on exit (does not actually implement save states).
- In order to avoid complexity, it does not support `GBC` games. And because the `Playdate` has a monochrome
display, it wouldn't be that useful.
- Does not implement sound emulation. I hope to work on this in the future, but I suspect it will be quite
computationally intensive to run.
- Passes all CPU, time and interrupt blargg tests.
- Does not emulate the bootrom, goes straight into the game entrypoint.
- Gives me a warm and fuzzy feeling every time I get nostalgic about Game Boy games! (Ok, this probably
does not belong on this list, but it's pretty much why I wrote it in the first place).

## How do I get it working?

Easy! You will need to follow a couple of steps, since I have not created any actual releases just yet.

### Supported build systems

I have tested these instructions susccessfully in the following machines:
- Macbook Air with M2 running macOS.
- PC running Ubuntu 22.04 on an AMD CPU.
- PC running Arch Linux on an AMD CPU.

Note that running the build on Asahi Linux is not really supported, since the Playdate SDK doesn't
have aarch64 linux support.

The build will probably work in other systems, it just hasn't been tested. You're welcome to submit
patches for your system if you find a problem.

### Dependencies

In summary, we will need:
- The `Playdate SDK`.
- A nightly `Rust` compiler, which is required by [`crankstart`](https://github.com/pd-rs/crankstart.git).
- The `SDL2` libraries, used for the host builds of `Rusty Boy`.

Let's start by downloading the `Playdate SDK` and extracting it in a known location.

```sh
wget https://download.panic.com/playdate_sdk/Linux/PlaydateSDK-2.4.2.tar.gz
tar -xf PlaydateSDK-2.4.2.tar.gz
```

We will need to get a copy of a nightly `Rust` compiler. I suggest you use [`rustup.rs`](https://rustup.rs)

```sh
rustup toolchain install nightly-2024-04-30
rustup target add thumbv7em-none-eabihf --toolchain nightly-2024-04-30
rustup component add rust-src --toolchain nightly-2024-04-30
```

Finally install `SDL2`. On `Ubuntu 22.04` you can do this with:

```sh
sudo apt-get install libsdl2-dev
```

### Building

`Rusty Boy` can be built for both the host and the target with:

```sh
# Run this command on the root directory of this repository
cargo xtask build --release
```

You will find the executable in:

```
./target/release/rusty-boy-sdl
```

Now you can run games with:

```
./target/release/rusty-boy-sdl <ROM_PATH>
```

The Rusty Date application will be located in `rusty-date/build/rusty_date.pdx`.
Load this application to your Play Date to run the emulator.

### Running tests

You can run all unit tests by simply executing:

```sh
cargo test
```

### Loading Rusty Date to your playdate

Follow the instructions for sideloading the game (`rusty_date.pdx`) as described
[here](https://help.play.date/games/sideloading/).

### Loading ROMs to the playdate

#### :warning: Disclaimer :warning:

>Please, use this emulator responsibly and only use it with legally obtained ROMs. I do not condone the
use of this software for piracy purposes. All responsibility for such actions will fall on the user.
By using this software you accept these terms and conditions.

Ok, I will assume from this point onwards that you have legally obtained your cartridge dumps. If so,
to upload them to the Playdate follow these instructions:

- Connect your Playdate via USB to your computer. Navigate to `Settings -> System -> Reboot to Data Disk`
on the Playdate and wait for the USB MSD device to show up as a disk on your compuer.
- Once the disk is mounted, drag and drop the cartrige ROMs you want to play to the `Data/Rusty Date`
directory. All ROMs must end with the `.gb` extension in order to be detected by `Rusty Date`.
- Eject the disk and wait until the Playdate reboots.
- You can now play your games! They should appear in a list when you first open the `Rusty Date` game.
