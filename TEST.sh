#!/bin/sh
mkdir -p bin
RUST_BACKTRACE=1 rustc --test -g -o bin/test src/vodk/main.rs -L extern/glfw-rs/lib -L extern/gl-rs/lib -C link-args="-lglfw -lrt -lXrandr -lXi -lGL -lm -ldl -lXrender -ldrm -lXdamage -lX11-xcb -lxcb-glx -lxcb-dri2 -lXxf86vm -lXfixes -lXext -lX11 -lpthread -lxcb -lXau" && ./bin/test