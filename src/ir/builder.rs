use super::*;

use std::rc::Rc;
use std::cell::RefCell;

pub struct IrFunctionBuilder {
    pub var: Binding,
    pub params: Vec<Binding>,
    pub body: Vec<Expr>,
    pub method: bool, // false by default
}

// This will very likely change in the near future
// ... a small experimental thing
impl IrFunctionBuilder {
    // the build-a-function way
    pub fn new_local(name: &str, depth: usize, function_depth: usize) -> Self {
        IrFunctionBuilder {
            var: Binding::local(name, depth, function_depth),
            params: Vec::new(),
            body: Vec::new(),
            method: false
        }
    }

    pub fn new_global(name: &str, function_depth: usize) -> Self {
        IrFunctionBuilder {
            var: Binding::global(name, function_depth),
            params: Vec::new(),
            body: Vec::new(),
            method: false
        }
    }

    pub fn from(var: Binding, params: Vec<Binding>, body: Vec<Expr>) -> Self {
        IrFunctionBuilder {
            var,
            params,
            body,
            method: false
        }
    }

    pub fn params(mut self, params: Vec<Binding>) -> Self {
        self.params = params;
        self
    }

    pub fn body(mut self, body: Vec<Expr>) -> Self {
        self.body = body;
        self
    }

    pub fn var(&self) -> &Binding {
        &self.var
    }

    pub fn build(self) -> IrFunction {
        let func_body = IrFunctionBody {
            params: self.params,
            method: self.method,
            inner:  self.body,
        };

        IrFunction {
            var: self.var,
            body: Rc::new(RefCell::new(func_body))
        }
    }
}

pub struct IrBuilder {
    info: Program,
    program: Vec<Expr>,
}

impl IrBuilder {
    pub fn new() -> Self {
        IrBuilder {
            info: Program::empty(),
            program: Vec::new(),
        }
    }

    pub fn with_entry(entry: DataId) -> Self {
        IrBuilder {
            info: Program::with_entry(entry),
            program: Vec::new(),
        }
    }

    pub fn ret(&mut self, value: Option<ExprNode>) {
        self.emit(
            Expr::Return(value)
        )
    }

    pub fn call(&mut self, callee: ExprNode, args: Vec<ExprNode>, retty: Option<TypeInfo>) -> ExprNode {
        let call = Call {
            callee,
            args
        };

        Expr::Call(call).node(
            if let Some(info) = retty {
                info
            } else {
                TypeInfo::none(true)
            }
        )
    }

    pub fn var(&self, binding: Binding) -> Expr {
        Expr::Var(binding)
    }

    pub fn mutate(&mut self, lhs: ExprNode, rhs: ExprNode) {
        self.emit(Expr::Mutate(lhs, rhs))
    }

    // Binding to be resolved manually
    pub fn bind(&mut self, binding: Binding, rhs: ExprNode) {
        self.emit( Expr::Bind(binding, rhs))
    }

    pub fn function(&mut self, func: IrFunction) {
        self.emit(Expr::Function(func))
    }

    // Binds a clean local binding, should be resolved after
    pub fn bind_local(&mut self, name: &str, rhs: ExprNode, depth: usize, function_depth: usize) -> Binding {
        let binding = Binding::local(name, depth, function_depth);

        self.bind(binding.clone(), rhs);

        binding
    }

    pub fn bind_global(&mut self, name: &str, rhs: ExprNode, depth: usize) -> Binding {
        let binding = Binding::global(name, depth);

        self.emit(Expr::BindGlobal(binding.clone(), rhs));

        binding
    }

    pub fn binary(&self, lhs: ExprNode, op: BinaryOp, rhs: ExprNode) -> ExprNode {
        Expr::Binary(lhs, op, rhs).node(TypeInfo::none(true))
    }

    pub fn unary(op: UnaryOp, rhs: ExprNode) -> Expr {
        Expr::Unary(op, rhs)
    }

    pub fn int(&mut self, n: i32) -> ExprNode {
        let info = TypeInfo::new(Type::Int, true);
        let lit = Literal::Number(n as f64);

        Expr::Literal(lit).node(info)
    }

    pub fn number(&mut self, n: f64) -> ExprNode {
        let info = TypeInfo::new(Type::Float, true);
        let lit = Literal::Number(n);

        Expr::Literal(lit).node(info)
    }

    pub fn string(&mut self, s: &str) -> ExprNode {
        let info = TypeInfo::new(Type::String, true);
        let lit = Literal::String(s.to_owned());

        Expr::Literal(lit).node(info)
    }

    pub fn bool(&mut self, b: bool) -> ExprNode {
        let info = TypeInfo::new(Type::Bool, true);
        let lit = Literal::Boolean(b);

        Expr::Literal(lit).node(info)
    }

    pub fn nil(&mut self) -> ExprNode {
        let info = TypeInfo::new(Type::Nil, true);
        let lit = Literal::Nil;

        Expr::Literal(lit).node(info)
    }

    pub fn build(self) -> Vec<Expr> {
        self.program
    }

    pub fn emit(&mut self, atom: Expr) {
        self.program.push(atom)
    }
}