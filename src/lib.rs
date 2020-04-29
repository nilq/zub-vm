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

        let mut value_ref = Binding::define_local("foo");
        value_ref.resolve(0, 0);

        builder.bind_global("FOO", builder.var(value_ref));

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

    #[test]
    fn actual_real_functions() {
        /*
            function foo(a, b) {
                return a + b
            }

            global bar = foo(10.0, 30.0)
        */

        let mut body_builder = IrBuilder::new(); // B)

        let a = body_builder.var(
                Binding::local("a", 1, 1)
            );

        let b = body_builder.var(
            Binding::local("b", 1, 1)
            );

        let sum = body_builder.binary(a, BinaryOp::Add, b);

        body_builder.ret(Some(sum));

        let func = IrFunctionBuilder::new_local("foo", 0, 0)
            .params(
                vec![
                    Binding::local("a", 1, 0),
                    Binding::local("b", 1, 0),
                ]
            )
            .body(body_builder.build())
            .build(); // You can just keep building, costs a few clone()s though



        let mut builder = IrBuilder::new();
        builder.function(func); // Declare function here


        let args = vec![
            builder.number(10.0),
            builder.number(30.0)
        ];

        let callee = builder.var(Binding::local("foo", 0, 0));
        let call = builder.call(callee, args, None);

        builder.bind_global("bar", call); // assign "bar" to call here

        let mut vm = VM::new();
        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }
}
