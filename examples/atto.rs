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
            if *ident == "if" {
                let cond = parse_expr(builder, slice, get_binding)?;
                let a = parse_expr(builder, slice, get_binding)?;
                let b = parse_expr(builder, slice, get_binding)?;
                Some(builder.if_(cond, a, Some(b)))
            } else if let Some(op) = match *ident {
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
            } else if let Some(val) = match *ident {
                "true" => Some(builder.bool(true)),
                "false" => Some(builder.bool(false)),
                "null" => Some(builder.nil()),
                _ => None,
            } {
                Some(val)
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
                Some((params.len(), Binding::define_global(ident)))
            } else if let Some((_, binding)) = params.iter().rev().find(|(name, _)| *name == &ident) {
                Some((0, binding.clone()))
            } else {
                get_binding(ident)
            })?;

            let mut weird_test_builder = IrBuilder::new();
            weird_test_builder.ret(Some(body));

            let f = IrFunctionBuilder::new_global(*name)
                .params(params.iter().map(|(_, b)| b).cloned().collect())
                .body(weird_test_builder.build())
                .build();

            Some((*name, params.len(), f))
        },
        _ => panic!("Not a function: {:?}", slice),
    }
}

const CODE: &str = r#"
fn sum x is
    if = x 0
        1
    + sum - x 1 sum - x 1

fn main is
    sum 12
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

    let build = builder.build();

    // println!("{:#?}", build);
    // println!();
    // println!();

    let mut vm = VM::new();
    vm.exec(&build);
    println!("{:?}", vm.globals["entry"]);
}
