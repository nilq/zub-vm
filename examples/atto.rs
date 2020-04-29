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
            let params = slice[2..]
                .into_iter()
                .take_while(|token| **token != "is")
                .map(|param| (param, Binding::define_local(param)))
                .collect::<Vec<_>>();
            *slice = &slice[params.len() + 3..];
            let mut builder = IrBuilder::new();
            let body = parse_expr(&mut builder, slice, &|ident| if ident == *name {
                Some((params.len(), Binding::define_local(ident)))
            } else if let Some((_, binding)) = params.iter().rev().find(|(name, _)| *name == &ident) {
                Some((0, binding.clone()))
            } else {
                get_binding(ident)
            })?;
            builder.ret(Some(body));
            let f = IrFunctionBuilder::new_global(*name)
                .params(params.iter().map(|(_, b)| b).cloned().collect())
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
    let mut fns = Vec::<(&str, usize, Binding)>::new();
    let mut token_slice = &tokens[..];
    while let Some((name, args, f)) = parse_fn(&mut builder, &mut token_slice, &|ident| {
        fns.iter().rev().find(|f| f.0 == ident).map(|f| (f.1, f.2.clone()))
    }) {
        builder.function(f);
        fns.push((name, args, Binding::define_global(name)));
    }

    let main_var = builder.var(Binding::define_global("main"));
    let main_call = builder.call(main_var, vec![], None);
    builder.bind_global("entry", main_call);

    let mut vm = VM::new();
    vm.exec(&builder.build());
    println!("{:?}", vm.globals["entry"]);
}
