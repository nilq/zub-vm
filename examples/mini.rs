// Mini Rust language
use zubbers::{ir::*, vm::*};

extern crate logos;
use logos::Logos;

use std::collections::HashMap;

#[derive(Logos, Debug, PartialEq, Clone)]
enum Token<'t> {
    #[regex("[0-9.]+")]
    Number(&'t str),
    #[regex("[a-zA-Z]+")]
    Ident(&'t str),
    #[token("fn")]
    Fun,
    #[token("global")]
    Global,
    #[token("let")]
    Let,
    #[token("if")]
    If,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LCurly,
    #[token("}")]
    RCurly,
    #[token("@")]
    Period,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("+")]
    Add,
    #[token("-")]
    Sub,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("=")]
    Assign,
    #[token("%")]
    Rem,
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl<'t> Token<'t> {
    fn to_op(&self) -> Option<Op> {
        use self::Token::*;

        Some(
            match *self {
                Add => Op::Add,
                Sub => Op::Sub,
                Mul => Op::Mul,
                Div => Op::Div,
                Rem => Op::Rem,
                Period => Op::Index,

                _ => return None
            }
        )
    }
}

#[derive(Debug, Clone)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Index,
}

impl Op {
    pub fn prec(&self) -> usize {
        use self::Op::*;

        match self {
            Add => 0,
            Sub => 0,
            Mul => 1,
            Div => 1,
            Rem => 1,
            Index => 4,
        }
    }

    pub fn to_ir(&self) -> BinaryOp {
        use self::Op::*;

        match self {
            Add => BinaryOp::Add,
            Sub => BinaryOp::Sub,
            Mul => BinaryOp::Mul,
            Div => BinaryOp::Div,
            Rem => BinaryOp::Rem,
            // TODO: Index BinOp deprecated
            _ => panic!("Index deprecated"),
        }
    }
}

#[derive(Debug, Clone)]
enum Statement {
    Let(String, Expression, Binding),
    Global(String, Expression),

    Fun(String, Vec<String>, Vec<Statement>, Binding),
    If(Expression, Vec<Statement>, Option<Vec<Statement>>),
    While(Expression, Vec<Statement>),
    Assign(Expression, Expression),
    Return(Option<Expression>),

    Expression(Expression)
}

#[derive(Debug, Clone)]
enum Expression {
    Number(f64),
    Binary(Box<Expression>, Op, Box<Expression>),
    Array(Vec<Expression>),
    Dict(Vec<Expression>, Vec<Expression>), // Don't care about hashmaps :p
    Var(String, Binding), // It will store the proper relative depth
    Call(Box<Expression>, Vec<Expression>),
}

struct Parser<'p> {
    tokens: Vec<Token<'p>>,
    ast: Vec<Statement>,

    top: usize,

    depth_table: HashMap<String, Binding>,
    depth: usize,
    function_depth: usize,

    in_operation: bool,
}

impl<'p> Parser<'p> {
    pub fn new(tokens: Vec<Token<'p>>) -> Self {
        Parser {
            tokens,
            ast: Vec::new(),
            top: 0,

            depth_table: HashMap::new(),
            depth: 0,
            function_depth: 0,

            in_operation: false
        }
    }

    pub fn parse(&mut self) -> Vec<Statement> {
        while self.remaining() > 0 {
            let statement = self.parse_statement();

            if let Some(s) = statement {
                self.ast.push(s)
            }
        }

        self.ast.clone()
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        use self::Token::*;

        match self.current() {
            Global => {
                self.next();

                let name = self.current_slice().unwrap().to_string();

                self.next();

                if self.current() == Assign {
                    self.next();

                    let right = self.parse_expression().unwrap();

                    self.depth_table.insert(name.clone(), Binding::global(name.as_str()));

                    Some(
                        Statement::Global(
                            name,
                            right,
                        )
                    )
                } else {
                    panic!("Expected `=`")
                }
            },

            Let => {
                self.next();

                let name = self.current_slice().unwrap().to_string();

                self.next();

                if self.current() == Assign {
                    self.next();

                    let right = self.parse_expression().unwrap();

                    let binding = Binding::local(name.as_str(), self.depth, self.function_depth);
                    self.depth_table.insert(name.clone(), binding.clone());

                    Some(
                        Statement::Let(
                            name,
                            right,
                            binding
                        )
                    )
                } else {
                    panic!("Expected `=`")
                }
            },

            Fun => {
                self.next();
                let name = self.current_slice().unwrap().to_string();

                let binding = Binding::local(name.as_str(), self.depth, self.function_depth);
                self.depth_table.insert(name.clone(), binding.clone());

                self.next();

                if self.current() == LParen {
                    self.next();

                    self.depth += 1;
                    self.function_depth += 1;

                    let mut params = Vec::new();

                    while self.current() != RParen {
                        let name = self.current_slice().unwrap().to_string();
                        params.push(name.clone());

                        let binding = Binding::local(name.clone().as_str(), self.depth, self.function_depth);
                        self.depth_table.insert(name, binding.clone());

                        self.next();

                        if self.current() == RParen {
                            break
                        }

                        if self.current() != Comma{
                            panic!("Expected `,` in function params, found {:?}", self.current())
                        }

                        self.next()
                    }

                    self.next(); // RParen


                    let body = self.parse_body();

                    self.depth -= 1;
                    self.function_depth -= 1;

                    Some(
                        Statement::Fun(
                            name,
                            params,
                            body,
                            binding
                        )
                    )

                } else {
                    panic!("Expected `(` in function")
                }
            },

            Return => {
                self.next();

                if self.current() == Semicolon {
                    Some(
                        Statement::Return(None)
                    )
                } else {
                    let a = Some(
                        Statement::Return(Some(self.parse_expression().unwrap()))
                    );

                    a
                }
            }

            Semicolon => {
                self.next();
                None
            }

            c => {
                let a = Some(
                    Statement::Expression(
                        self.parse_expression().unwrap()
                    )
                );

                a
            },
        }
    }

    fn parse_body(&mut self) -> Vec<Statement> {
        use self::Token::*;

        if self.current() != LCurly {
            panic!("Expected `{`")
        }

        self.next();

        let mut body = Vec::new();

        while self.current() != RCurly {
            let statement = self.parse_statement();

            if let Some(s) = statement {
                body.push(s)
            }
        }

        self.next();

        body
    }

    fn parse_expression(&mut self) -> Option<Expression> {
        use self::Token::*;

        let cur = self.current();

        let expr = match cur {
            Number(ref n) => {
                Expression::Number(
                    n.clone().parse::<f64>().unwrap()
                )
            },
            Ident(ref n) => {
                if let Some(depth) = self.depth_table.get(&n.to_string()) {
                    let mut binding = depth.clone();

                    if binding.depth.is_some() {
                        binding.depth = Some(self.depth);
                    }

                    let var = Expression::Var(
                        n.to_string(),
                        binding,
                    );

                    self.next();

                    if self.current() == LParen {
                        self.next();

                        let mut args = Vec::new();

                        while self.current() != RParen {
                            args.push(self.parse_expression().unwrap());

                            if self.current() == RParen {
                                break
                            }
    
                            if self.current() != Comma{
                                panic!("Expected `,` in call args, found {:?}", self.current())
                            }

                            self.next();
                        }

                        self.next();

                        Expression::Call(
                            Box::new(var),
                            args
                        )
                    } else {
                        var
                    }
                } else {
                    panic!("Can't find variable `{}`", n)
                }
            },

            LParen => {
                self.next();

                let flag = self.in_operation;
                self.in_operation = false;

                let expr = self.parse_expression().unwrap();

                self.in_operation = flag;

                if self.current() != RParen {
                    panic!("Expected `)` to close `(`");
                }

                expr
            },

            LCurly => {
                self.next();

                let mut keys = Vec::new();
                let mut vals = Vec::new();

                while self.current() != RCurly {
                    keys.push(self.parse_expression().unwrap());
                    
                    if self.current() != Colon {
                        panic!("Expected `:` after key")
                    }

                    self.next();

                    vals.push(self.parse_expression().unwrap());

                    if self.current() == RCurly {
                        break
                    }

                    if self.current() != Comma {
                        panic!("Expected `,` after value but found `{:?}`", self.current())
                    }

                    self.next();
                }

                Expression::Dict(keys, vals)
            }

            c => { println!("{:?}", c); self.next(); return None},
        };

        self.next();

        if self.remaining() == 0 {
            return Some(expr)
        }

        if self.current().to_op().is_some() && !self.in_operation {
            Some(
                self.parse_binary(expr)
            )
        } else {
            Some(expr)
        }
    }

    fn parse_binary(&mut self, left: Expression) -> Expression {
        use self::Token::*;
        
        let mut expr_stack = vec!(left);
        let mut op_stack   = vec!(self.current().to_op().unwrap());
        self.next();

        self.in_operation = true; // Don't want to chain operations

        expr_stack.push(self.parse_expression().unwrap());

        while op_stack.len() > 0 {
            while let Some(op) = self.current().to_op() {
                self.next();
                let precedence = op.prec();

                if precedence <= op_stack.last().unwrap().prec() {
                    let right = expr_stack.pop().unwrap();
                    let left  = expr_stack.pop().unwrap();

                    expr_stack.push(
                        Expression::Binary(
                            Box::new(left),
                            op_stack.pop().unwrap(),
                            Box::new(right)
                        )
                    );

                    if self.remaining() > 0 {
                        expr_stack.push(self.parse_expression().unwrap());
                        op_stack.push(op);
                    } else {
                        panic!("Reached EOF in binary operation")
                    }
                } else {
                    expr_stack.push(self.parse_expression().unwrap());
                    op_stack.push(op)
                }
            }

            let right = expr_stack.pop().unwrap();
            let left  = expr_stack.pop().unwrap();

            expr_stack.push(
                Expression::Binary(
                    Box::new(left),
                    op_stack.pop().unwrap(),
                    Box::new(right)
                )
            );
        }

        self.in_operation = false;

        expr_stack.pop().unwrap()
    }

    fn remaining(&self) -> usize {
        if self.top > self.tokens.len() {
            return 0
        }

        self.tokens.len() - self.top
    }

    fn next(&mut self) {
        self.top += 1
    }

    fn current(&self) -> Token {
        self.tokens[self.top.clone()].clone()
    }

    fn current_slice(&self) -> Option<&str> {
        use self::Token::*;

        match self.current() {
            Number(ref s) |
            Ident(ref s) => Some(s),
            _ => None
        }
    }

    fn peek(&self) -> Token {
        self.tokens[self.top + 1].clone()
    }
}

fn codegen_expr(builder: &IrBuilder, expr: &Expression) -> ExprNode {
    use self::Expression::*;

    match expr {
        Number(ref n) => {
            builder.number(*n)
        },

        Var(name, depth) => {
            builder.var(depth.clone())
        },

        Call(ref callee, ref args) => {
            let mut args_ir = Vec::new();

            for arg in args.iter() {
                args_ir.push(codegen_expr(&builder, arg))
            }

            let callee_ir = codegen_expr(&builder, callee);

            builder.call(callee_ir, args_ir, None)
        },

        Binary(left, op, right) => {
            let left  = codegen_expr(&builder, left);
            let right = codegen_expr(&builder, right);

            builder.binary(left, op.to_ir(), right)
        },

        Dict(keys, values) => {
            let mut keys_ir = Vec::new();
            let mut vals_ir = Vec::new();

            for key in keys.iter() {
                keys_ir.push(codegen_expr(&builder, key))
            }

            for value in values.iter() {
                vals_ir.push(codegen_expr(&builder, value))
            }

            builder.dict(keys_ir, vals_ir)
        },

        _ => todo!()
    }
}

fn codegen(builder: &mut IrBuilder, ast: &Vec<Statement>) {
    use self::Statement::*;
    
    for s in ast.iter() {
        match s {
            Let(name, expr, var) => {
                let right = codegen_expr(&builder, expr);
                builder.bind(var.clone(), right)
            },

            Global(name, expr) => {
                let right = codegen_expr(&builder, expr);
                builder.bind(Binding::global(name), right)
            },

            Fun(name, params, body, var) => {
                let params = params.iter().map(|x| x.as_str()).collect::<Vec<&str>>();

                let fun = builder.function(var.clone(), &params.as_slice(), |mut builder| {
                    codegen(&mut builder, body)
                });

                builder.emit(fun);
            },

            Return(ref val) => {
                let value = if let Some(v) = val {
                    Some(
                        codegen_expr(&builder, v)
                    )
                } else {
                    None
                };

                builder.ret(value)
            },

            Expression(ref expr) => {
                let expr = codegen_expr(&builder, expr);
                builder.emit(expr)
            },

            c => todo!("{:#?}", c)
        }
    }
}

const TEST: &'static str = r#"
let bar = 13.37;

fn foo() {
  fn baz(c) {
    return c + bar;
  }

  return baz(10);
}

global gangster = foo();
"#;

fn main() {
    let lex = Token::lexer(TEST);

    let mut parser = Parser::new(lex.collect::<Vec<Token>>());

    let ast = parser.parse();

    let mut builder = IrBuilder::new();
    codegen(&mut builder, &ast);

    let ir = builder.build();

    println!("{:#?}", ir);

    let mut vm = VM::new();
    vm.exec(&ir, true);

    println!("{:#?}", vm.globals)
}