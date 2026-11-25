<div align="center">

# shitty-hdl

_There are many HDLs like it, but this one is mine._

</div>

An experimental HDL combining concepts from [Bluespec][bsc], [Amaranth][amaranth], and [Spade][spade], packaged as a cohesive set of Rust crates.


> [!NOTE]
> The user-facing high-level API is the focus of this experiment; all the internal details and compiler code is AI-generated.

# Motivation

There's no point using a tool if you don't understand the design decisions and trade-offs that it makes. **If you read nothing else here, read this section.**

HDLs generally come in two kinds: the "so ubiquitous that they're literally compilation targets" (VHDL, SystemVerilog), and everything else. Everything[^1] can compile to SystemVerilog, so it's very much the Assembly of the hardware world[^2], but SV -- much like Assembly -- works at a level too low for mere mortals to program against effectively. That's why higher-level HDLs exist to provide some capability that can't be expressed in SV alone, but so far there has not emerged a coherent "good HDL" (in my opinion). This is my attempt at designing a better one, stealing the features I think fit well.

[Bluespec][bsc] is probably the closest relative here, as it gives you the tools to think about hardware _transactions_ rather than _registers_ -- a good high-level abstraction[^3] -- and has pioneered the concept of guarded atomic actions. In fact, our HDL compiles to Bluespec SystemVerilog! It's the first HDL where I felt that this could be "to SystemVerilog what Rust is to assembly"[^2], but it struggles in two aspects:
- expressing FSMs and coroutines
- language tooling

These are the two areas I aim to improve with my design.

## Guarded atomic actions and coroutines

While Bluespec does a great job of letting you define rules for hardware transactions, it's still not great for expressing how these rules call one another. If you want to represent the hardware taking a series of actions, with some unknown number of clock cycles (variable latency) in between each action, you have to define an `enum State { Idle, Step1, Step2, ... }` and write the FSM manually. This is tedious, somewhat error-prone, and makes it hard to understand the flow of the code at a glance[^4]. Fortunately, Rust gives us a tool to handle this for free: the compiler turns `async` functions into a state machine, so why can't do that as well?

In fact, after a bit of research I found that [TxHDL] does almost exactly this. Many of my design decisions were almost the same as what they implemented, such as making a Rust library instead of a new HDL; using types to represent clock domains; and using Rust's `async` machinery natively. I swear I had no clue about them before I wrote this section, but it's good to see that we converged on similar designs independently!

By using Rust's own async/await machinery to express "hardware async" (I mean that's basically what it is[^5]), you can write something that looks like Bluespec's guarded atomic actions, but compose like Rust async calls. For example, a simple memory allocator with a free list would look like:

```rs
type PageAddr = U<45>;

let alloc: Method<(), PageAddr>;
//                ^^  ^^^^^^^^
//                in  out

let free_head = Reg::new(clock, u!(0));
let free_list_len = Reg::new(clock, u!(0));

let alloc = Method::define_as(async |()| {
    if free_list_len > 0 {
        // take the head of the free list
        
        let next_free_head = memory.read(free_head).await;
        //                         clock cycle here ^^^^^

        free_head <<= next_free_head;
        //        ^^^ assign to a register
        free_list_len <<= free_list_len - 1;

        // return the current `free_head` from this `Method`
        free_head
    }
});
```

No need for an `enum State` (although you could still write one if you wanted to), and control flow is very clear.

## Tooling

### Language

Writing a new HDL from scratch means that you will have to write your own parser, lexer, compiler, IR, LSP... but that can be side-stepped by piggybacking off of an existing language. [TxHDL] is essentially a DSL in Rust; we do the same. We don't have to reimplement the expressive type system, const generics, compiler, LSP, diagnostics, letting us iterate on the core HDL API faster.

### Boards

Amaranth is the HDL in which I was able to bring up a blinking light demo the fastest. It's so ubiquitous as a hardware hello-world that it even has a blinking module built into the library. [amaranth-boards] gives you premade definitions of what properties an FPGA has, what pins are used for buttons, what's used for LEDs, etc. covering a good range of common hardware. Plus, the platform abstraction lets you say "I want to wire this signal to an LED" without needing to care what the underlying pin is, or writing a Xilinx `.xdc` file -- the tooling handles all of that for you. It's a massive boon to testing, and a DX I wanted to replicate in my HDL; the result is this:

```rs
#[module]
pub fn my_module(platform: Platform) {
    let clk = platform.clock::<SysClk>();

    let start = platform.button.take(0).input(clk);
    let mut busy_led = platform.led.take(0).output(clk);
    let mut result_leds = [
        platform.led.take(1).output(clk),
        platform.led.take(2).output(clk),
        platform.led.take(3).output(clk),
    ];

    Rule::define_as(|| {
        busy_led <<= start;
    });
}
```

[^1]: Okay not exactly. Amaranth lowers to a netlist, then to Yosys RTLIL first, but that can be lowered to SV.
[^2]: https://speakerdeck.com/bcantrill/software-co-design-the-coming-golden-age?slide=41
[^3]: https://cacm.acm.org/practice/abstraction-in-hardware-system-design/
[^4]: https://uwplse.org/2026/03/24/hw-coroutines.html
[^5]: https://www.hdlfactory.com/post/2026/09/12/txhdl-a-hardware-description-language-that-is-a-rust-library/

[bsc]: https://github.com/B-Lang-org/bsc
[amaranth]: https://github.com/amaranth-lang/amaranth
[spade]: https://spade-lang.org/
[TxHDL]: https://github.com/filmil/hdl-txhdl
[amaranth-boards]: https://github.com/amaranth-lang/amaranth-boards
