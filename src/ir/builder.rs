use super::*;


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

    // Binding to be resolved manually
    pub fn bind(&mut self, id: LocalId, binding: Binding, rhs: AtomNode) {
        self.emit( Atom::Bind(id, binding, rhs))
    }

    // Binds a clean local binding, should be resolved after
    pub fn bind_local(&mut self, id: LocalId, name: &str, rhs: AtomNode, depth: usize, function_depth: usize) {
        let mut binding = Binding::local(name);

        binding.resolve(depth, function_depth);

        self.bind(id, binding, rhs)
    }

    pub fn bind_global(&mut self, name: &str, rhs: AtomNode) {
        let binding = Binding::global(name);

        self.emit(Atom::Global(binding, rhs))
    }

    pub fn binary(lhs: AtomNode, op: BinaryOp, rhs: AtomNode) -> Atom {
        Atom::Binary(lhs, op, rhs)
    }

    pub fn unary(op: UnaryOp, rhs: AtomNode) -> Atom {
        Atom::Unary(op, rhs)
    }

    fn emit(&mut self, atom: Atom) {
        self.program.push(atom)
    }
}