use super::*;

use std::rc::Rc;
use std::cell::RefCell;

pub struct IrFunctionBuilder {
    pub var: Binding,
    pub params: Vec<Binding>,
    pub body: Vec<Atom>,
    pub method: bool, // false by default
}

// This will very likely change in the near future
// ... a small experimental thing
impl IrFunctionBuilder {
    // the build-a-function way
    pub fn new_local(name: &str) -> Self {
        IrFunctionBuilder {
            var: Binding::local(name),
            params: Vec::new(),
            body: Vec::new(),
            method: false
        }
    }

    pub fn from(var: Binding, params: Vec<Binding>, body: Vec<Atom>) -> Self {
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

    pub fn body(mut self, body: Vec<Atom>) -> Self {
        self.body = body;
        self
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
    program: Vec<Atom>,
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

    pub fn ret(&mut self, value: Option<AtomNode>) {
        self.emit(
            Atom::Return(value)
        )
    }

    pub fn call(&mut self, callee: AtomNode, args: Vec<AtomNode>, retty: Option<TypeInfo>) -> AtomNode {
        let call = Call {
            callee,
            args
        };

        Atom::Call(call).node(
            if let Some(info) = retty {
                info
            } else {
                TypeInfo::none(true)
            }
        )
    }

    pub fn mutate(&mut self, lhs: AtomNode, rhs: AtomNode) {
        self.emit(Atom::Mutate(lhs, rhs))
    }

    // Binding to be resolved manually
    pub fn bind(&mut self, binding: Binding, rhs: AtomNode) {
        self.emit( Atom::Bind(binding, rhs))
    }

    // Binds a clean local binding, should be resolved after
    pub fn bind_local(&mut self, name: &str, rhs: AtomNode, depth: usize, function_depth: usize) {
        let mut binding = Binding::local(name);

        binding.resolve(depth, function_depth);

        self.bind(binding, rhs)
    }

    pub fn bind_global(&mut self, name: &str, rhs: AtomNode) {
        let binding = Binding::global(name);

        self.emit(Atom::BindGlobal(binding, rhs))
    }

    pub fn binary(lhs: AtomNode, op: BinaryOp, rhs: AtomNode) -> Atom {
        Atom::Binary(lhs, op, rhs)
    }

    pub fn unary(op: UnaryOp, rhs: AtomNode) -> Atom {
        Atom::Unary(op, rhs)
    }

    pub fn int(&mut self, n: i32) -> AtomNode {
        let info = TypeInfo::new(Type::Int, true);
        let lit = Literal::Number(n as f64);

        Atom::Literal(lit).node(info)
    }

    pub fn number(&mut self, n: f64) -> AtomNode {
        let info = TypeInfo::new(Type::Float, true);
        let lit = Literal::Number(n);

        Atom::Literal(lit).node(info)
    }

    pub fn string(&mut self, s: &str) -> AtomNode {
        let info = TypeInfo::new(Type::String, true);
        let lit = Literal::String(s.to_owned());

        Atom::Literal(lit).node(info)
    }

    pub fn bool(&mut self, b: bool) -> AtomNode {
        let info = TypeInfo::new(Type::Bool, true);
        let lit = Literal::Boolean(b);

        Atom::Literal(lit).node(info)
    }

    pub fn nil(&mut self) -> AtomNode {
        let info = TypeInfo::new(Type::Nil, true);
        let lit = Literal::Nil;

        Atom::Literal(lit).node(info)
    }

    pub fn build(self) -> Vec<Atom> {
        self.program
    }

    fn emit(&mut self, atom: Atom) {
        self.program.push(atom)
    }
}