# Quantum Kaledioscope

Following the example of the famous (infamous?) [LLVM Kaledioscope tutorial](https://llvm.org/docs/tutorial/) and its many derivatives, in the next few chapters we'll walk through how to define your own quantum language, interpret and simulate quantum programs written in your new language, and the basics of how to compile quantum programs.

There's a lot of good quantum languages out there already (Q# being a particular favorite), so our Quantum Kaledioscope (let's call it QK for short) is not really going to wind up being practical on its own, but hopefully going through the steps needed to parse, interpret, and compile quantum programs will be helpful in understanding quantum compilation stacks more generally.

Throughout the next few chapters, we'll be building up our Quantum Kaledioscope tooling as a Rust command-line tool. You can run and experiment with the tool by using `cargo`:

<!-- TODO: It's more than a parser now, may want to move this folder. -->
<!-- TODO: Update as more subcommands are added. -->
```text
$ git clone https://github.com/qrust-dev/quantinomocon.git
$ cd quantinomocon/code/parser
$ cargo run -- --help
USAGE:
    parser.exe <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    build-ast    Parses a Quantum Kalediscope program and prints an abstract syntax tree for
                 the program
    help         Print this message or the help of the given subcommand(s)
    interpret    Interprets a Quantum Kalediscope program and runs it on a full-state quantum
                 simulator
    parse        Parses a Quantum Kalediscope program and prints the result
```
