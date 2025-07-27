# cargo-buckal

Seamlessly build Cargo packages with Buck2.

## Install

```
git clone https://github.com/r2cn-dev/cargo-buckal.git
cd cargo-buckal
cargo install --path .
```

## Usage

```
Usage: cargo-buckal <COMMAND>

Commands:
  build  Compile the current package
  init   Create a new package in an existing directory
  new    Create a new package
  clean  Clean up the buck-out directory
  add    Add dependencies to a manifest file
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```