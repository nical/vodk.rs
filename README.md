vodk.rs
=======

Rust experiments, mostly around game programming related stuff.

Mostly compatible with rust:
    rustc 0.13.0-nightly (ffc111889 2014-12-12 21:07:19 +0000)

Building
========

Note: Actually in order to build you have to invok `cargo build` in each sub-crate.
This will redownload and compile each dependancies.

It's quite hacky. This will be fixed in the future.

First build
-----------

### Dependancies

(Cargo)[https://crates.io/] take care of the dependancies.

#### Legacy:

Vodk currently depends on this Rust crates.

-    https://github.com/nical/gl-rs
-    https://github.com/nical/glfw-rs
-    ~~https://github.com/nical/rust-png~~ removed.
-    ~~https://github.com/kballard/rust-lua.git~~ not yet.

Troubleshooting(s)
------------------

Old version of OpenGL are not supported.
