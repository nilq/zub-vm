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
            } else if let Ok(n) = ident.parse() {
                Some(builder.number(n))
            } else if let Some((args, binding)) = get_binding(ident) {
                let args = (0..args).map(|_| parse_expr(builder, slice, get_binding)).collect::<Option<_>>()?;
                Some(builder.call(
                    builder.var(binding),
                    args,
                    None,
                ))
            } else {
                None
            }
        },
    }
}

fn parse_fn<'a>(
    builder: &mut IrBuilder,
    slice: &mut &'a [&'a str],
    get_binding: &impl Fn(&str) -> Option<(usize, Binding)>,
) -> Option<(&'a str, usize, IrFunction)> {
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
            let mut builder = IrBuilder::new();
            let body = parse_expr(&mut builder, slice, &|ident| if ident == *name {
                Some((params.len(), Binding::define_local(ident)))
            } else if let Some(binding) = params.get(&ident) {
                Some((0, binding.clone()))
            } else {
                get_binding(ident)
            })?;
            builder.ret(Some(body));
            let f = IrFunctionBuilder::new_global(*name)
                .params(params.values().cloned().collect())
                .body(builder.build())
                .build();
            Some((*name, params.len(), f))
        },
        _ => panic!("Not a function: {:?}", slice),
    }
}

const CODE: &str = r#"
    fn add x y is + x y

    fn sub x y is - x y

    fn main is
        add 5 sub 7 4
"#;

fn main() {
    let tokens = CODE.split_whitespace().collect::<Vec<_>>();

    let mut builder = IrBuilder::new();
    let mut fns = HashMap::<_, (usize, _)>::new();
    let mut token_slice = &tokens[..];
    while let Some((name, args, f)) = parse_fn(&mut builder, &mut token_slice, &|name| {
        fns.get(name).cloned()
    }) {
        builder.function(f);
        fns.insert(name, (args, Binding::define_global(name)));
    }

    let main_var = builder.var(Binding::define_global("main"));
    let main_call = builder.call(main_var, vec![], None);
    builder.bind_global("main2", main_call);

    let mut vm = VM::new();

    let ir = builder.build();
    //println!("{:#?}", ir);
    vm.exec(&ir);

    println!("{:?}", vm.globals);
}
