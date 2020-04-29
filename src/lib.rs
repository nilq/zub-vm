pub mod vm;
pub mod ir;
pub mod compiler;

#[cfg(test)]
mod tests {
    use super::vm::*;
    use super::ir::*;

    #[test]
    fn globals() {
        let mut builder = IrBuilder::new();

        let value = builder.number(42.0);
        builder.bind_global("foo", value);

        let mut vm = VM::new();

        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn locals() {
        let mut builder = IrBuilder::new();

        let value = builder.number(42.0);
        builder.bind_local("foo", value, 0, 0);

        let mut value_ref = Binding::local("foo");
        value_ref.resolve(0, 0);

        builder.bind_global("FOO", builder.var(value_ref).node(TypeInfo::none(true)));

        let mut vm = VM::new();

        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn binary() {
        let mut builder = IrBuilder::new();

        let a = builder.number(20.0);
        let b = builder.number(30.0);

        let sum = builder.binary(a, BinaryOp::Add, b);

        builder.bind_global("sum", sum);

        let mut vm = VM::new();
        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }
}
