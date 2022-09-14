# Gload
A "better" download button for your Github repository.

## A new download button
This project is a "attempt" to make downloading from Github repositories (specifically those hosting Rust projects as of right now). After reading about some none technical peoples opinions about downloading software from Github I realized how much us more tecnical people take for granted in terms of building from source (but also how much more simple it is than downloading binaries).
This led me to lazily start work on Gload, a way to build the project from source and then send the executable straight to the user, without them having to touch a version number or build tool.

## Usage
```
gload 0.1.0

USAGE:
    gload.exe [OPTIONS] <repo>

ARGS:
    <repo>    The repo to compile and distribute

OPTIONS:
    -d, --debug               Toggled debug output
    -h, --help                Print help information
    -p, --path [<path>...]    The path to place "repo_to_compile" in. (defauls to "./"
    -t [<timeout>...]         How long values should live (in seconds) in the cache! Set to 0 for no cache timeout. (defaults to 1024 seconds)
    -V, --version             Print version information
```

## How it works
Gload is implemented as a simple webserver which simply reads information from the connecting users machine to reliably compile for their computer architecture.
After this Gload compiles the project for that specific architecture and stores it in a cache for easy access for subsequent users and returns the executable file to the client.
For hosting on lowend machines, its possible to change the lifetime of the data in the cache to offset CPU cycles (through compilation) against storage space (the compiled binaries stored on disk and in cache).

## Disclaimer
This is by no means meant to *actually* be a better download button, obviously it has all kinds of issues such as trust and speed (and most likely security). This was just a fun project to do to learn more about `Axum` and async Rust. If you think it looks cool and your users wont get spooked by getting sent to a shady white page, then by all means use it. Otherwise just compile the executables inside your CI pipeline and link to the executable from your README.

## Bugs and issues:
* If there are dependencies needed to compile the project these need to be installed before running `Gload` or if running through docker these need to be handled in other ways.
* On firefox the request might timeout if the compilation takes a long time. This can be fixed by going to `about:config` and setting `network.notify.changed` to `false`.