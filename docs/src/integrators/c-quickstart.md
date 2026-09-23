# C quickstart

The native package installs the public C header, shared library, CMake package,
a buildable example, and a small deterministic media fixture. Rust and Cargo are
not required on the consuming machine.

Assuming PostProject is installed under `/opt/postproject`, build and run the
installed example with:

```sh
cmake \
  -S /opt/postproject/share/doc/postproject/examples/c \
  -B build/postproject-c-example \
  -DCMAKE_PREFIX_PATH=/opt/postproject \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/postproject-c-example --config Release
ctest --test-dir build/postproject-c-example \
  --build-config Release --output-on-failure
```

The test creates `c-example.pproj`, imports the installed
`sample-media.dat`, commits the transaction, and verifies that the asset owns
one representation. Its production path must not already exist.

For an application target, consume the same package normally:

```cmake
find_package(PostProject 0.3 REQUIRED CONFIG)
target_link_libraries(my_application PRIVATE PostProject::postproject)
```

The example source demonstrates explicit error-handle ownership, transaction
commit, result-set ownership, and production cleanup. The full contract for
every function remains in `<postproject/postproject.h>`.
