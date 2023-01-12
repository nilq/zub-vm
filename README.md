# Zub VM
> A super-fast, stack-based virtual machine for dynamic languages

# Warning
This library has recently been forked and is being reworked on to add new features, don't expect a lot of stuff to be stable.

## Features

- NaN-tagging value representation
- Mark n' sweep garbage collection
- Compact bytecode format
- Easy-to-use intermediate representation

## Milestones

- [x] Refined VM based on work by [Mr Briones](https://github.com/cwbriones)
- [x] Tracing garbage collector
- [x] High-level IR
- [x] Compilation of IR
- [ ] Optimizer (currently 80-90% Python speed, aiming for much faster)
- [x] Profiler and disassembler

## Example

### Building IR is easy

Getting your backend up and running shouldn't have to be hard.

The following code builds IR for evaluating `sum = 20.0 + 30.0`:

```rust
let mut builder = IrBuilder::new();

let a = builder.number(20.0);
let b = builder.number(30.0);

let sum = builder.binary(a, BinaryOp::Add, b);

builder.bind(Binding::global("sum"), sum);
```

When you feel like the IR is looking smooth. Simply let VM throw it through the compiler, and run it.

```rust
let mut vm = VM::new();
vm.exec(&builder.build());
```

## Languages

### Hugorm

Hugorm is a dynamic, python-like language being built for small data science and game projects.

[https://github.com/nilq/hugorm](https://github.com/nilq/Hugorm)

### Examples

The `examples/` folder includes two small language implementations running on the ZubVM.

#### Atto

Atto is a functional, minimal language that showcases how little code is needed to implement a working, Turing-complete language. The syntax can be seen in the following teaser:

```hs
fn sum x is
    if = x 0
        1
    + sum - x 1 sum - x 1

fn main is
    sum 12
```

#### Mini

Mini is a simple language that looks basically like a mix of Rust and JavaScript. It covers a bit wider set of features than Atto. This does show in the size of the language though.

```rust
let bar = 13.37;

fn foo() {
  fn baz(c) {
    return c + bar;
  }
  
  return baz(10);
}

global gangster = foo();
```


## Special thanks

- [zesterer](https://github.com/zesterer)
- [cwbriones](https://github.com/cwbriones)
- [evolbug](https://github.com/evolbug)
