# Parsing with pest.rs

## Writing the parser

<!-- TODO: list lines in Cargo.toml for pest? -->

Our first step in building a compiler for Quantum Kaledioscope will be to write a parser that takes raw text for a quantum program, and that outputs a data structure that we can work with in further compiler stages. To do so, we'll use the [`pest` crate](https://pest.rs/), as `pest` offers us a nice _macro_ that we can use to greatly simplify writing quantum programs.

Indeed, using `pest`, our parser comes out to be very simple indeed:

```rust
{{#include ../../../code/parser/src/parser.rs:7:9}}
```

Believe it or not, that's all the Rust code we need to write entire parser. The `#[derive(Parser)]` macro provided by `pest` takes care of all the hard work of writing a fast and robust parser in Rust for us; all we need to do at this point is specify the grammar, indicated by `qkaledioscope.pest`.

<!-- TODO: settle I vs we here. -->
OK, I lied: writing the parser isn't the first step. We need to decide what Quantum Kaledioscope even _is_, so that we can write the grammar we need to pass to `#[derive(Parser)]`! It's helpful to recall some of the basic elements we need in any quantum programming language, as we saw in [the chapter on quantum programming](../quantum-programming/overview.md):

- We need a way to call intrinsic quantum operations like `H` and `X`.
- We need to be able to refer to individual qubits in our device.
- We need to be able to measure qubits and extract classical data.
- We need a way to represent flow control on classical data.

In Quantum Kaledioscope, we'll provide these elements by modeling a quantum program as a sequence of one or more _functions_, each of which can call quantum gates defined as prototypes. For example, we might write a simple QRNG program in QK as:

```text
{{#include ../../../code/parser/examples/qrng.qk}}
```

Above, we've defined each gate that we have access to with an `extern` declaration, and have defined one function `qmain` by using the `def` keyword. That `qmain` function then refers to particular qubits (e.g.: `#0`) made available by our device, and passes those qubits to the various gates exposed by our simulator or device.

In our grammar, we may represent this by a sequence of _rules_ that tell `pest` how to turn the various elements in our QK program above into data structures we can work with from Rust. For example, a program is a sequence of zero or more declarations and definitions, so we can write that out explicitly in our `pest` grammar:

```text
program = _{ SOI ~ (file_element)* ~ EOI }

file_element = _{ (declaration | definition) }
declaration = { Extern ~ prototype ~ Semicolon }
definition = { Def ~ prototype ~ definition_body }
```

We'll need some additional rules to specify what text should be processed for `Extern`, `prototype`, and other symbols that we needed to use in our definition of the `program` rule. Generally, these rules are broken down into those rules that check against a string directly (terminal rules), and those that can recurse into other rules (non-terminal). By convention, we'll write out terminal rules in our grammar using `PascalCase` and non-terminals as `snake_case`.

<!-- TODO: Describe if and while to connect to the fourth essential we need for quantum languages. -->

For the full listing of the grammar for QK, check out <https://github.com/qrust-dev/quantinomocon/blob/main/code/parser/src/qkaledioscope.pest>.

## Calling into the parser

- [ ] Describe using `std::fs` to load a string.
- [ ] The `Pair<Rule>` structure that comes back.
- [ ] Using the `pretty-print` feature to turn to JSON and print.
- [ ] Wrapping everything with `clap`.

```text
$ cargo run parse .\examples\qrng.qk
   Compiling parser v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.28s
     Running `target\debug\parser.exe parse .\examples\qrng.qk`
{
  "pos": [
    0,
    130
  ],
  "pairs": [
    {
      "pos": [
        0,
        20
      ],
      "rule": "declaration",
      "inner": {
        "pos": [
          7,
          19
        ],
        "pairs": [
          {
            "pos": [
              7,
              19
            ],
            "rule": "prototype",
            "inner": {
              "pos": [
                7,
                19
              ],
              "pairs": [
                {
                  "pos": [
                    7,
                    8
                  ],
                  "rule": "Ident",
                  "inner": "h"
                  ...
```
