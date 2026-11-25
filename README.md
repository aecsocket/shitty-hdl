# Crates

## Elaboration

- `front`: the user-facing, high-level API types such as `Signal<T>`, `Module`, integer types, packed types, and control flow; lowers to `pir`.
- `pir`: the program IR (modules and statements); lowers to `nir`.
- `nir`: the netlist IR (nets, cells, and wires); lowers to `rtlil`.
- `rtlil`: how to lower `nir` into [Yosys RTLIL][rtlil].
- `irt`: shared IR crate utilities.

## Hardware

- `board`: definitions for physical board hardware, such as what LEDs or buttons a board has, what pins correspond to each resource, and what toolchain to use to program a board.
- `platform`: turns a `board` definition into `front` items; gives you a `Platform` from which you can request LEDs, buttons, etc. during elaboration.
- `toolchain`: exposes board-specific tools such as Yosys, Vivado, nextpnr, which can build/upload/verify your [RTLIL][rtlil].

## Umbrella

- `cli`: CLI logic for interacting with the toolchain (build, upload, etc.) for your board and design.
- `derive`: derive macros for the whole crate.
- `.`: wraps all of the above into one tidy package for you!

[rtlil]: https://yosyshq.readthedocs.io/projects/yosys/en/stable/yosys_internals/formats/rtlil_rep.html

# Notes

general ideas:
- very similar to amaranth in semantics, except there is no inherent `sync` clock domain; you have to pass a `clk: Clock` and `clk.on(..)`
- lowers to verilog because everything lowers to verilog
- core datatypes:
  - boolean
    - `bool`: single bit/wire
    - `Bits<const N: usize>`: _N_ bits packed together
    - these types have no arithmetic operators
  - integer
    - `Uint<const N: usize>`: _N_-wide unsigned integer
      - the most generic, unoptimized type; use only when you need something outside the range `u2` to `u128` 
      - note: unoptimized for _simulation_; lowers to the same verilog (`u<64> == u64` in verilog)
    - `Sint<const N: usize>`: _N_-wide signed integer
    - `(u|i)(2..=128)` i.e. `u67`, `i45`: optimized forms of integers
    - these have arithmetic operators
