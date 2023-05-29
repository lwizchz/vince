Copyright (c) 2023 Louise Montalvo <louanmontalvo@gmail.com>

# Vince Audio-Video Synthesizer

This is a modular synthesizer system built to process both audio and video
signals. It is in early development so please do not expect stability.

Modules are defined in individual files under the `src/modules` directory. Each
module can take several inputs, produce several outputs, and use several "knobs"
which are used to adjust module-specific parameters such as gain or speed.

## Usage

Be sure to run the program in release mode since development mode has poor
performance. Also memory will start to balloon since the output buffers won't
be consumed quickly enough.

```
$ cargo run --release racks/rack0.toml
```

# Racks

Racks consist of modules and the patches between them. They are defined as TOML
files under the `assets/racks` directory which is relative to either the
`CARGO_MANIFEST_DIR` or the built executable. See the provided racks for
details on how to make your own. When a rack file is modified, it will be
hot-reloaded without needing to restart the program.

Basic example rack:

```toml
[modules]
0 = { name = "Audio Out", type = "AudioOut", knobs = [1.0] }

1 = { type = "Oscillator", func = "Sine", knobs = [0.0, 440.0, 1.0, 0.0] }
2 = { type = "Oscilloscope" }

[patches]
1M0O = [ # Take module 1's output 0
    "0M0I", # And patch it here to module 0's input 0
    "2M0I", # And here to module 2's input 0
]
```

Each module is keyed by an index. These indices are not necessarily sequential
which allows for easy commenting out of modules during testing. Each module has
a type which specifies the name of the struct defined in the module source.
Optionally, each module can be named. This name will appear on screen in place
of the module type. The remaining parameters are module-specific so be sure to
read each module's documentation to understand what each one does.

A patch consists of a key defining the output index and an array that lists the
input indices that the given output should be copied to. Each IO index is
specific to a certain module, so it also contains the module index. Patches can
also be created between outputs and knobs. See `racks/rack1.toml` for an
example.
