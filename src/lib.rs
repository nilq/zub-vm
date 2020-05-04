extern crate flame;
#[macro_use] extern crate flamer;

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
        builder.bind(Binding::global("foo"), value);

        let mut vm = VM::new();

        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn locals() {
        let mut builder = IrBuilder::new();

        let value = builder.number(42.0);
        builder.bind(Binding::local("foo", 0, 0), value);

        builder.bind(Binding::global("FOO"), builder.var(Binding::local("foo", 0, 0)));

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

        builder.bind(Binding::global("sum"), sum);

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
        
        let foo = builder.function(Binding::local("foo", 0, 0), &["a", "b"], |builder| {

            let a = builder.var(Binding::local("a", 1, 1));
            let b = builder.var(Binding::local("b", 1, 1));

            let sum = builder.binary(a, BinaryOp::Add, b);

            builder.ret(Some(sum))
        });

        builder.emit(foo);

        let args = vec![
            builder.number(10.0),
            builder.number(30.0)
        ];

        let callee = builder.var(Binding::local("foo", 0, 0));
        let call = builder.call(callee, args, None);

        builder.bind(Binding::global("bar"), call); // assign "bar" to call here

        let built = builder.build();

        let mut vm = VM::new();
        vm.exec(&built, true);

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn ffi() {
        let mut builder = IrBuilder::new();

        let hello = Expr::Literal(
            Literal::String("Hello from Rust :D".to_string())
        ).node(TypeInfo::new(Type::String));
        
        let callee = builder.var(Binding::global("print"));

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

        builder.bind(Binding::local("bob", 0, 0), list);

        let var = builder.var(Binding::local("bob", 0, 0));

        let index = builder.int(0);
        
        let new_element = builder.number(777.0);
        let set_element = builder.list_set(var.clone(), index.clone(), new_element);
        builder.emit(set_element);

        let right = builder.list_get(var, index);

        builder.bind(Binding::global("element"), right); // expect 777.0

        let mut vm = VM::new();
        vm.exec(&builder.build());

        println!("{:#?}", vm.globals)
    }

    #[test]
    fn recursion() {
        let mut builder = IrBuilder::new();

        let fib_binding = Binding::local("fib", 0, 0);
        let fib = builder.function(fib_binding.clone(), &["n"], |builder| {
            let upvalue_fib = Binding::local("fib", 1, 0);

            let n = builder.var(
                Binding::local("n", 1, 1)
            );

            let one = builder.number(1.0);
            let two = builder.number(2.0);

            let binary_0 = builder.binary(n.clone(), BinaryOp::Sub, one);
            let binary_1 = builder.binary(n.clone(), BinaryOp::Sub, two);
            
            println!("{}", upvalue_fib.is_upvalue());

            let fib_var = builder.var(upvalue_fib.clone()); // Fine for now, always pointing in the right direction :D
            let call_0 = builder.call(fib_var.clone(), vec![binary_0], None);
            let call_1 = builder.call(fib_var, vec![binary_1], None);


            let final_binary = builder.binary(call_0, BinaryOp::Add, call_1);

            let three = builder.number(3.0);
            let n_less_than_3 = builder.binary(n.clone(), BinaryOp::LtEqual, three);
            let ternary = builder.ternary(n_less_than_3, n.clone(), Some(final_binary));

            builder.ret(Some(ternary))
        });

        builder.emit(fib);

        let ten = builder.number(10.0);
        let fib_var = builder.var(fib_binding);

        let fib_call = builder.call(fib_var, vec![ten], None);

        let print = builder.var(Binding::global("print"));
        let call  = builder.call(print, vec!(fib_call), None);

        builder.emit(call); // :D

        fn print_native(heap: &Heap<Object>, args: &[Value]) -> Value {
            println!("{}", args[1].with_heap(heap));
            Value::nil()
        }

        let mut vm = VM::new();
        vm.add_native("print", print_native, 1);
        vm.exec(&builder.build());
    }
}
