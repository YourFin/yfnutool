# yfnutool

Tree-sitter based nushell fiddling. Right now just implements a half-finished (but usable) "do string interpolation at point" command.

# Features

## DWIM Interpolate (ctrl-s)

<video src="https://github.com/user-attachments/assets/cc389005-f0d2-4112-8910-5778c28b33bd"></video>

# Installing

## Source build
Building:

``` nushell
git clone --recurse-submodules https://github.com/YourFin/yfnutool.git
cd yfnutool
cargo install --path .
# Make sure that the cargo install dir is in $env.PATH
_yfnutool-bin --help
# Add yfnutool to your loadable modules
$"\n\$env.NU_LIB_DIRS += (pwd)/nu-mod/yfnutool\n" | save --append ~/.config/nushell/env.nu
```

## Nix

Here's the commit where I installed this in my own dotfiles: https://github.com/YourFin/dotfiles/commit/13cf732c5f3f1a0c9273cbc712e0eda1b19a8483

## Usage

You'll need to set up the keybinding yourself. For the recommended configuration (Ctrl-s -> `yfnutool interpolate`), you'll want to add:

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

Most of the testing infrastructure uses a string representation encoding the cursor, with the cursor position represented by the first `|` character in the string. You can try this by passing such a string to with the `--test-string` argument:

``` text
$ cargo run -- --test-string "hello 'worl|d'"
hello $'worl(|)d'
```

To help with writing tree-matching code, at higher log levels the binary will dump the parsed syntax tree: 

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

The `_yfnutool-bin` binary expects a [MsgPack](https://msgpack.org/) two-element array via stdin:

``` text
[ cursor position (in unicode graphemes from start), command line text (utf-8) ]
```

and returns the same structure via stdout. The "unicode graphemes from start" is what [`commandline get-cursor`](https://www.nushell.sh/commands/docs/commandline_get-cursor.html) returns.

The nu module in [./nu-mod](./nu-mod) wires this into [`commandline`](https://www.nushell.sh/commands/docs/commandline.html).
