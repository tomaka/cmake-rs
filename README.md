cmake-rs
========

This library allows you to invoke **CMake** during the compilation of a Rust program or library. This is very useful when you want to link C or C++ libraries with your Rust libraries.

Example
-------

Let's say that you want to create a library that links to C library which you can compile with CMake.

First, include the source code of the C library in your repository (usually with a git submodule).

```
mylib
|~c_library
| |~src
| | | ...
| | CMakeLists.txt
|~src
| | mylib.rs
| Cargo.toml
```

Then, add `cmake-rs` as a dependency to your project.

```toml
[dependencies.cmake]
git = "http://github.com/Tomaka17/cmake-rs"
```

Import `cmake-rs` your main rust file:

```rust
#![feature(phase)]

#[phase(plugin)]
extern crate cmake;
```

And finally, call the `cmake!` macro somewhere in the code:

`cmake!("c_library")`

The argument of the macro is the location of the `CMakeLists.txt`.

How does it work?
-----------------

The `cmake!` macro will invoke CMake and output all libraries to `<path_of_lib>/cmake-build-result`. It will use `<path_of_lib>/cmake-build` as the build directory.

`<path_of_lib>/cmake-build-result` is then added to the list of directories where the Rust compiler will look for libraries.
**You still need to add `#[link(name="lib")]` inside your project.**
