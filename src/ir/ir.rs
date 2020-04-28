use super::TypeInfo;

use std::collections::HashMap;

pub type LocalId = usize;
pub type DataId  = usize;

pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
}

// When depth is None, we're dealing with a global.
pub struct Binding {
    name: String,
    depth: Option<usize>,
    function_depth: usize,
}

impl Binding {
    pub fn global(name: &str) -> Self {
        Binding {
            name: name.to_string(),
            depth: None,
            function_depth: 0,
        }
    }

    pub fn local(name: &str) -> Self {
        Binding {
            name: name.to_string(),
            depth: Some(0),
            function_depth: 0
        }
    }

    pub fn resolve(&mut self, depth: usize, function_depth: usize) {
        self.depth = Some(depth);
        self.function_depth = function_depth
    }

    #[inline]
    pub fn is_upvalue(&self) -> bool {
        self.depth
            .map(|d| d > self.function_depth)
            .unwrap_or(false)
    }

    pub fn upvalue_depth(&self) -> Option<usize> {
        self.depth.and_then(|d|
            if self.is_upvalue() {
                Some(d - self.function_depth)
            } else {
                None
            })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NEq,
    GtEq,
    LtEq,
    Gt,
    Lt,
    And,
    Or,
}

pub enum UnaryOp {
    Neg,
    Not,
}

pub struct Function {
    arg_count: usize,
    params: Vec<Binding>,
}

pub struct Call {
    pub callee: Node<Atom>,
    pub args: Vec<Node<Atom>>,
}

pub struct Node<T> {
    inner: Box<T>,
    type_info: TypeInfo,
}

impl<T> Node<T> {
    pub fn new(inner: T, type_info: TypeInfo) -> Self {
        Node {
            inner: Box::new(inner),
            type_info
        }
    }
}

pub type AtomNode = Node<Atom>;

pub enum Atom {
    Data(DataId),

    Literal(Literal),
    Bind(LocalId, Binding, AtomNode), // @zesterer: like `with` 
    Global(Binding, AtomNode),
    Mutate(AtomNode, AtomNode),
    Binary(AtomNode, BinaryOp, AtomNode),
    Call(Call),
    Function(Function),
    Unary(UnaryOp, AtomNode),
    Return(Option<AtomNode>),

    If(AtomNode, AtomNode, Option<AtomNode>),
    While(AtomNode, AtomNode),

    Break,
}

impl Atom {
    pub fn node(self, type_info: TypeInfo) -> AtomNode {
        Node::new(self, type_info)
    }
}

pub struct Program {
    data: HashMap<DataId, AtomNode>,
    entry: Option<DataId>
}

impl Program {
    pub fn empty() -> Self {
        Program {
            data: HashMap::new(),
            entry: None,
        }
    }

    pub fn with_entry(entry: DataId) -> Self {
        Program {
            data: HashMap::new(),
            entry: Some(entry)
        }
    }

    pub fn insert(&mut self, id: DataId, atom: AtomNode) {
        self.data.insert(id, atom);
    }
}