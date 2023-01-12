use super::TypeInfo;

use std::{
    collections::HashMap,
    rc::Rc,
    cell::RefCell,
    fmt,
};

pub type LocalId = usize;
pub type DataId  = usize;

#[derive(Clone, Debug)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
}

// When depth is None, we're dealing with a global.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub name: String,
    pub depth: Option<usize>,
    pub function_depth: usize,
}

impl Binding {
    // Define to be resolved later
    pub fn define_local(name: &str) -> Self {
        Binding {
            name: name.to_string(),
            depth: Some(0),
            function_depth: 0
        }
    }

    pub fn global(name: &str) -> Self {
        Binding {
            name: name.to_string(),
            depth: None,
            function_depth: 0
        }
    }

    pub fn local(name: &str, depth: usize, function_depth: usize) -> Self {
        Binding {
            name: name.to_string(),
            depth: Some(depth),
            function_depth: function_depth
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

#[derive(Clone, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Equal,
    NEqual,
    GtEqual,
    LtEqual,
    Gt,
    Lt,
    And,
    Or,
    Pow,
}

#[derive(Clone, Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub struct IrFunctionBody {
    pub params: Vec<Binding>,
    pub method: bool,
    pub inner: Vec<ExprNode>, // the actual function body
}

#[derive(Clone, Debug)]
pub struct IrFunction {
    pub var: Binding,
    pub body: Rc<RefCell<IrFunctionBody>>, // A Literal/Constant
}

#[derive(Clone, Debug)]
pub struct Call {
    pub callee: Node<Expr>,
    pub args: Vec<Node<Expr>>,
}

#[derive(Clone)]
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

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#?}", self.inner)
    }
}

pub type ExprNode = Node<Expr>;

// NOTE: LocalId removed for now, as it wasn't used in the compiler


#[derive(Clone, Debug)]
pub enum Expr {
    Data(DataId),

    Literal(Literal),

    Bind(Binding, ExprNode), // @zesterer: like `with`
    BindGlobal(Binding, ExprNode),

    Var(Binding), // access binding

    Mutate(ExprNode, ExprNode),
    Binary(ExprNode, BinaryOp, ExprNode),
    Call(Call),
    Function(IrFunction),
    AnonFunction(IrFunction), // variable here will be unique id
    Unary(UnaryOp, ExprNode),
    Return(Option<ExprNode>),

    Not(ExprNode),
    Neg(ExprNode),

    If(ExprNode, ExprNode, Option<ExprNode>),
    While(ExprNode, ExprNode),

    List(Vec<ExprNode>),
    Dict(Vec<ExprNode>, Vec<ExprNode>), // They need to be the same size, funny enough
    SetElement(ExprNode, ExprNode, ExprNode),
    GetElement(ExprNode, ExprNode),

    Block(Vec<ExprNode>),

    Break,
    Pop,
}

impl Expr {
    pub fn node(self, type_info: TypeInfo) -> ExprNode {
        Node::new(self, type_info)
    }
}

#[derive(Debug)]
pub struct Program {
    data: HashMap<DataId, ExprNode>,
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

    pub fn insert(&mut self, id: DataId, atom: ExprNode) {
        self.data.insert(id, atom);
    }
}
