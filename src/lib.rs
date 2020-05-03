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
        builder.bind("foo", value);

        let mut vm = VM::new();

        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn locals() {
        let mut builder = IrBuilder::new();

        let value = builder.number(42.0);
        builder.bind("foo", value);

        builder.bind("FOO", builder.var("foo"));

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

        builder.bind("sum", sum);

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

        let mut builder = IrBuilder::new();
        
        let foo = builder.function("foo", &["a", "b"], |builder| {

            let a = builder.var("a");
            let b = builder.var("b");

            let sum = builder.binary(a, BinaryOp::Add, b);

            builder.ret(Some(sum))
        });

        builder.emit(foo);

        let args = vec![
            builder.number(10.0),
            builder.number(30.0)
        ];

        let callee = builder.var("foo");
        let call = builder.call(callee, args, None);

        builder.bind("bar", call); // assign "bar" to call here

        let built = builder.build();

        let mut vm = VM::new();
        vm.exec(&built);

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn ffi() {
        let mut builder = IrBuilder::new();

        let hello = Expr::Literal(
            Literal::String("Hello from Rust :D".to_string())
        ).node(TypeInfo::new(Type::String));
        
        let callee = builder.var("print");
        let call = builder.call(callee, vec!(hello), None);

        builder.emit(call);

        fn print(heap: &Heap<Object>, args: &[Value]) -> Value {
            println!("{}", args[1].with_heap(heap));
            Value::nil()
        }


        let mut vm = VM::new();

        vm.add_native("print", print, 1);
        vm.exec(&builder.build());
    }

    #[test]
    fn list() {
        let mut builder = IrBuilder::new();

        let content = vec![
            builder.number(11.0),
            builder.number(22.0),
            builder.number(33.0),
        ];

        let list = builder.list(content);

        builder.bind("bob", list);

        let var = builder.var("bob");

        let index = builder.int(0);
        
        let new_element = builder.number(777.0);
        let set_element = builder.list_set(var.clone(), index.clone(), new_element);
        builder.emit(set_element);

        let right = builder.list_get(var, index);

        builder.bind("element", right); // expect 777.0

        let mut vm = VM::new();
        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }
}
