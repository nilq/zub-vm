# Zub VM
> A super-fast, stack-based virtual machine for dynamic languages

## Example

### Building IR is easy

Getting your backend up and running shouldn't have to be hard.
The following code builds IR for evaluating `sum = 20.0 + 30.0`.

```rust
let mut builder = IrBuilder::new();

let a = builder.number(20.0);
let b = builder.number(30.0);

let sum = builder.binary(a, BinaryOp::Add, b);

builder.bind_global("sum", sum);
```

When you feel like the IR is looking smooth. Simply let VM throw it through the compiler, and run it.

```rust
let mut vm = VM::new();
vm.exec(&builder.build());
```

## Milestones

- [x] Refined VM based on work by [Mr Briones](https://github.com/cwbriones)
- [x] Tracing garbage collector
- [x] High-level IR
- [x] Compilation of IR
- [ ] Optimizer
- [ ] Profiler and disassembler


## Special thanks

- [zesterer](https://github.com/zesterer)
- [cwbriones](https://github.com/cwbriones)
