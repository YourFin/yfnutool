# yfnutool

Tree-sitter based nushell fiddling. Right now just implements a half-finished (but usable) "do string interpolation at point" command.

# Installing

## Source build
Building:

``` nushell
git clone --recurse-submodules https://github.com/YourFin/yfnutool.git
cd yfnutool
cargo install --path .
# Make sure that the cargo install dir is in $env.PATH
yfnutool --help
# Add yfnutool.nix to the vendor directory
$"\n\$env.NU_LIB_DIRS += (pwd)/nu-mod/yfnutool" | save --append ~/.config/nushell/env.nu
# Load the yfnutool module in your config
"\nuse yfnutool *\n" | save --append ~/.config/nushell/env.nu
```

## Nix

TODO

## Using

You'll need to set up the keybinding yourself. For the recommended configuration (Ctrl-s -> yfnutool interpolate), you'll want to add:

``` nushell
use yfnutool *
$env.config.keybindings ++= [
  { name: yfnutool_interpolate
  , modifier: CONTROL
  , keycode: Char_s
  , mode: vi_insert # Alternatively, "emacs"
  , event: 
    { send: executehostcommand
    , cmd: 'yfnutool interpolate' 
    } 
  } 
]
```

to your `config.nu`.

# Developing
## Testing the nix build

``` sh
nix-build --expr 'with import <nixpkgs> {}; callPackage (import ./.) {}'
```

## Testing

Most of the testing infrastructure is built around a string encoding of the command line. This representation encodes the cursor position with a `|` character in the string. You can try this by passing such a string to the `yfnutool` binary with the `--test-string` argument:

``` text
$ yfnutool --test-string "hello 'worl|d'"
hello $'worl(|)d'
# Alternatively
$ cargo run -- --test-string "hello 'worl|d'"
hello $'worl(|)d'
```

At higher log levels, the binary will dump the parsed syntax tree, which can be useful for understanding how to 

``` text
$[2025-01-12T00:59:12Z TRACE yfnutool] info: Tree sitter parse results:
      |
    1 | hello 'world'
      | -------------
      | |     |
      | |     info: 55AC61309560 (5): val_string
      | info: 55AC6130A4F0 (4): cmd_identifier
      | info: 55AC61309620 (3): command
      | info: 55AC61309680 (2): pipe_element
      | info: 55AC6130B130 (1): pipeline
      | info: 55AC61309740 (0): nu_script
      |
    help: 55AC61309560 (5): val_string (kind_id: 440)
      |
    1 | hello 'world'
      |       ------- help: here (byte 6-13)
      |
    help: 55AC6130A4F0 (4): cmd_identifier (kind_id: 334)
      |
    1 | hello 'world'
      | ----- help: here (byte 0-5)
      |
    help: 55AC61309620 (3): command (kind_id: 461)
      |
    1 | hello 'world'
      | ------------- help: here (byte 0-13)
      |
    help: 55AC61309680 (2): pipe_element (kind_id: 390)
      |
    1 | hello 'world'
      | ------------- help: here (byte 0-13)
      |
    help: 55AC6130B130 (1): pipeline (kind_id: 321)
      |
    1 | hello 'world'
      | ------------- help: here (byte 0-13)
      |
    help: 55AC61309740 (0): nu_script (kind_id: 309)
      |
    1 | hello 'world'
      | ------------- help: here (byte 0-13)
      |
[2025-01-12T00:59:12Z DEBUG yfnutool] Single quote string
[2025-01-12T00:59:12Z TRACE yfnutool] Escaping parens
hello $'worl(|)d' yfnutool -vvv --test-string "hello 'worl|d'"
```

## Wiring

The `yfnutool` binary expects a [MsgPack](https://msgpack.org/) two-element array via stdin:

``` text
[ cursor position (in unicode graphemes from start), command line text (utf-8) ]
```

and returns the same structure. The "unicode graphemes from start" is what [`commandline get-cursor`](https://www.nushell.sh/commands/docs/commandline_get-cursor.html) returns.

