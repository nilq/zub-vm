use std::collections::HashMap;
use zub::{ir::*, vm::*};

fn parse_expr(
    builder: &mut IrBuilder,
    slice: &mut &[&str],
    get_binding: &impl Fn(&str) -> Option<(usize, Binding)>,
) -> Option<Node<Expr>> {
    match *slice {
        [] => None,
        [ident, ..] => {
            *slice = &slice[1..];
            if let Some((args, binding)) = get_binding(ident) {
                let args = (0..args).map(|_| parse_expr(builder, slice, get_binding)).collect::<Option<_>>()?;
                Some(builder.call(
                    builder.var(binding),
                    args,
                    None,
                ))
            } else if let Ok(n) = ident.parse() {
                Some(builder.number(n))
            } else {
                if let Some(op) = match *ident {
                    "+" => Some(BinaryOp::Add),
                    "-" => Some(BinaryOp::Sub),
                    "*" => Some(BinaryOp::Mul),
                    "/" => Some(BinaryOp::Div),
                    //"%" => Some(BinaryOp::Rem),
                    "=" => Some(BinaryOp::Equal),
                    ">" => Some(BinaryOp::Gt),
                    "<" => Some(BinaryOp::Lt),
                    ">=" => Some(BinaryOp::GtEqual),
                    "<=" => Some(BinaryOp::LtEqual),
                    "&" => Some(BinaryOp::And),
                    "|" => Some(BinaryOp::Or),
                    _ => None,
                } {
                    let a = parse_expr(builder, slice, get_binding)?;
                    let b = parse_expr(builder, slice, get_binding)?;
                    Some(builder.binary(a, op, b))
                } else {
                    None
                }
            }
        },
    }
}

fn parse_fn<'a>(
    builder: &mut IrBuilder,
    slice: &mut &'a [&'a str],
    get_binding: &impl Fn(&str) -> Option<(usize, Binding)>,
) -> Option<(&'a str, usize, Node<Expr>)> {
    match *slice {
        [] => None,
        ["fn", name, ..] => {
            *slice = &slice[2..];
            let mut params = HashMap::new();
            slice
                .into_iter()
                .take_while(|token| **token != "is")
                .for_each(|param| {
                    params.insert(param, Binding::define_local(param));
                    *slice = &slice[1..];
                });
            *slice = &slice[1..];
            let body = parse_expr(builder, slice, &|ident| if ident == *name {
                Some((params.len(), Binding::define_local(ident)))
            } else if let Some(binding) = params.get(&ident) {
                Some((0, binding.clone()))
            } else {
                get_binding(ident)
            })?;
            let f = IrFunctionBuilder::new_global(*name)
                .params(params.values().cloned().collect())
                .body(vec![body])
                .build();
            Some((*name, 0, builder.function(f)))
        },
        _ => panic!("Not a function"),
    }
}

const CODE: &str = r#"
    fn add x y is + x y

    fn sub x y is - x y

    fn main is
        + 5 - 7 4
"#;

fn main() {
    let tokens = CODE.split_whitespace().collect::<Vec<_>>();

    let mut builder = IrBuilder::new();
    let mut fns = HashMap::new();
    let mut token_slice = &tokens[..];
    while let Some((name, args, f)) = parse_fn(&mut builder, &mut token_slice, &|name| fns.get(name).cloned()) {
        println!("Defined {}", name);
        let binding = builder.bind_global(name, f);
        fns.insert(name, (args, binding));
    }

    let main_var = builder.var(Binding::define_global("main"));
    let main_call = builder.call(main_var, vec![], None);
    builder.bind_global("main2", main_call);

    let mut vm = VM::new();
    vm.exec(&builder.build());

    println!("{:?}", vm.globals);
}
