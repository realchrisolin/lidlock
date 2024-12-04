LidLock
======
Lock Windows laptop when lid is closed


## Introduction
This application is a simple rewrite of the C application of the same name (which itself is a rewrite of a C++ application), but rewritten in Rust -- originally designed to mimic the same behavior as macOS when one needs a laptop not to sleep but just to be locked when closing the lid.

The Rust version should work with clamshell mode and external monitors. It should ignore locking if lid is open and external monitors are connected, and reliably re-locks if the lid is closed and external monitors are disconnected.

## Usage
LidLock is a single portable executable file. Simply double-click the executable and it silently runs as a daemon in the background without any windows, prompts or icons. It listens to relevant events and does not consume CPU when waiting.

If you want to stop LidLock, you can use Task Manager to stop the process, or run the following command in cmd:
```cmd
taskkill /f /im lidlock.exe /t
```

To make it run at startup, create a shortcut of `lidlock.exe` and copy the shortcut to the startup folder (can be opened by executing `shell:startup` in Win+R).

## Download and Compilation
The pre-compiled binaries can be found at [Releases][release] page.

The binary can be cross-compiled using stable and nightly Rust toolchains as follows:

stable:
```bash
cargo build --release
```

nightly (optimized)
```bash
cargo +nightly build --release -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort
```

Compared to the C implementation which uses -O2 optimizations and compiles to ~30KB, this implementation (as of December 2024) is slightly less computationally efficient and compiles to ~95KB when using nightly build optimizations (~225KB without)

For debugging, add a flag `--debug` as a single argument when running the binary or specify path to log.
Default log path is %TEMP\lidlock.log

```cmd
lidlock.exe --debug
```
or
```cmd
lidlock.exe C:\custom.log
```

## Supplemental information

This app was written solely as a learning experience and posterity during the 2024 end-of-year holidays by a college dropout and systems administrator with a self-taught conceptual grasp of C and X86 assembly, an intermediate grasp of Python and OOP fundamentals, and no prior experience with Rust. PRs and bug reports are welcome.

## Credits
This app is based on [`lidlock`][lidlock] by @linusyang92 and rewritten in Rust.

The license is the same as `lidlock` (GPLv3):

[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

[lidlock]: https://github.com/linusyang92/lidlock
[laplock]: https://github.com/dechamps/laplock
