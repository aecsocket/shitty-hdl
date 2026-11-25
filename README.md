# Architecture

- `board` defines constants about different physical FPGA boards, such as which pins to use for LEDs, buttons; what toolchains you can use to program it
- `cli` lets you easily use the full toolchain from your own crate, by calling into the `cli`
- `core` provides primitives such as `Signal`, `Module`, integer types, which can be elaborated into program IR (`pir`)
- `derive` derive macro definitions for `core`
- `examples` examples on how to use this crate
- `pir` defines the layout of program IR, elaborated from `core` primitives
- `piri` lets you interpret and simulate `pir`, like `miri` does for rust `mir`
- `nir` defines 

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
