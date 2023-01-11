use super::super::gc::{tag::*, trace::*, *};
use super::*;

use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::rc::Rc;

use im_rc::hashmap::HashMap;

// lol nice
macro_rules! impl_as (
    ($name:ident, $typ:ident) => {
        pub fn $name(&self) -> Option<&$typ> {
            if let Object::$typ(ref o) = *self {
                Some(o)
            } else {
                None
            }
        }
    }
);

pub enum Object {
    String(String),
    Function(Function),
    NativeFunction(NativeFunction),
    Closure(Closure),
    List(List),
    Dict(Dict),
}

impl Object {
    impl_as!(as_string, String);
    impl_as!(as_closure, Closure);
    impl_as!(as_function, Function);
    impl_as!(as_list, List);
    impl_as!(as_dict, Dict);

    pub fn native_fn(
        name: &str,
        arity: u8,
        function: NativeFunctionType,
    ) -> Self {
        Object::NativeFunction(NativeFunction {
            name: name.into(),
            arity,
            function,
        })
    }

    pub fn as_closure_mut(&mut self) -> Option<&mut Closure> {
        if let Object::Closure(ref mut o) = *self {
            Some(o)
        } else {
            None
        }
    }
}

impl Trace<Self> for Object {
    fn trace(&self, tracer: &mut Tracer<Self>) {
        use self::Object::*;

        match self {
            String(_) => {}
            Function(f) => f.trace(tracer),
            NativeFunction(_) => {}
            Closure(c) => c.trace(tracer),
            List(l) => l.trace(tracer),
            Dict(d) => d.trace(tracer),
        }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use self::Object::*;

        match self {
            String(ref s) => write!(f, "{:?}", s),
            NativeFunction(ref na) => write!(f, "<native fn {:?}>", na.name),
            Function(ref fun) => write!(f, "{}", display_function(fun)),
            Closure(ref cl) => write!(f, "<closure {:?}>", cl.function),
            List(ref ls) => write!(f, "<list [{:?}]>", ls.content.len()),
            Dict(ref dict) => write!(f, "<dict [{:?}]>", dict.content.len()),
        }
    }
}

impl<'h, 'a> Display for WithHeap<'h, &'a Object> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use self::Object::*;

        match self.item {
            String(ref s) => write!(f, "{}", s),
            NativeFunction(ref na) => write!(f, "<native fn {}>", na.name),
            Function(ref fun) => write!(f, "{}", display_function(fun)),
            Closure(ref cl) => write!(f, "<fn {}>", cl.function.name),
            List(ref ls) => write!(f, "<list [{}]>", ls.content.len()),
            Dict(ref ls) => write!(f, "<dict [{}]>", ls.content.len()),
        }
    }
}

fn display_function(function: &Function) -> String {
    let chars = b"abcdefghijklmnopqrstuvwxyz";

    format!(
        "<fn {:}({:})>",
        function.name,
        (0..function.arity)
            .map(|num| (chars[num as usize] as char).to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[derive(Debug)]
pub struct FunctionBuilder {
    name: String,
    pub chunk: Chunk,
    arity: u8,
    upvalue_count: usize,
}

impl FunctionBuilder {
    pub fn new(name: &str, arity: u8) -> Self {
        let name: String = name.into();
        let chunk = Chunk::new(name.clone());
        FunctionBuilder {
            name,
            arity,
            chunk,
            upvalue_count: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    pub fn set_upvalue_count(&mut self, count: usize) {
        self.upvalue_count = count;
    }

    pub fn build(self) -> Function {
        Function::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    name: String,
    chunk: Chunk,
    arity: u8,
    upvalue_count: usize,
}

impl Function {
    fn new(builder: FunctionBuilder) -> Self {
        Function {
            name: builder.name,
            arity: builder.arity,
            chunk: builder.chunk,
            upvalue_count: builder.upvalue_count,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub fn upvalue_count(&self) -> usize {
        self.upvalue_count
    }
}

impl Trace<Object> for Function {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        self.chunk.trace(tracer);
    }
}

/// Type to represent a native function which is able to call
/// a function passed to it as an argument
pub type Callable<'a> = &'a mut dyn FnMut(Handle<Object>, u8, Vec<Value>);

/* #[derive(Clone)]
/// Whether the native function call will be able to call objects (functions)
/// passed as arguments
pub enum FunctionType {
    ParameterCall(fn(&mut Heap<Object>, &[Value], Callable) -> Value),
    Simple(fn(&mut Heap<Object>, &[Value]) -> Value),
}
 */

pub type NativeFunctionType = fn(&mut CallContext) -> Value;
#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: u8,
    pub function: NativeFunctionType,
}

#[derive(Debug, Clone)]
pub struct UpValue {
    inner: Rc<RefCell<Result<Value, usize>>>,
}

impl UpValue {
    pub fn new(local: usize) -> Self {
        UpValue {
            inner: Rc::new(RefCell::new(Err(local))),
        }
    }

    pub fn close<F: FnOnce(usize) -> Value>(&mut self, f: F) {
        let mut inner = self.inner.borrow_mut();
        if let Err(e) = *inner {
            *inner = Ok(f(e))
        }
    }

    pub fn as_local(&self) -> Option<usize> {
        self.inner.borrow().err()
    }

    pub fn get(&self) -> Result<Value, usize> {
        self.inner.borrow().clone()
    }

    pub fn set(&mut self, value: Value) -> Result<(), usize> {
        let mut inner = self.inner.borrow_mut();
        (*inner)?;

        *inner = Ok(value);

        Ok(())
    }
}

pub struct Dict {
    pub content: HashMap<HashValue, Value>,
}

impl Dict {
    #[inline]
    pub fn new(content: HashMap<HashValue, Value>) -> Self {
        Dict { content }
    }

    #[inline]
    pub fn empty() -> Self {
        Dict {
            content: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: HashValue, value: Value) {
        self.content.insert(key, value);
    }

    pub fn get(&self, key: &HashValue) -> Option<&Value> {
        self.content.get(key)
    }
}

impl Trace<Object> for Dict {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        self.content.values().for_each(|v| v.trace(tracer));
    }
}

#[derive(Debug)]
pub struct List {
    pub content: Vec<Value>,
}

// Inline everything >:()
impl List {
    #[inline]
    pub fn new(content: Vec<Value>) -> Self {
        List { content }
    }

    #[inline]
    pub fn set(&mut self, idx: usize, value: Value) {
        self.content[idx] = value
    }

    #[inline]
    pub fn push(&mut self, value: Value) {
        self.content.push(value)
    }

    #[inline]
    pub fn pop(&mut self) -> Value {
        self.content.pop().unwrap()
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Value {
        self.content[idx].clone() // Might not have to use a clone here
    }
}

impl Trace<Object> for List {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        self.content.iter().for_each(|v| v.trace(tracer));
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    function: Function,
    upvalues: Vec<UpValue>,
}

impl Closure {
    pub fn new(function: Function, upvalues: Vec<UpValue>) -> Self {
        Closure { function, upvalues }
    }

    pub fn name(&self) -> &str {
        self.function.name()
    }

    pub fn arity(&self) -> u8 {
        self.function.arity
    }

    pub fn chunk(&self) -> &Chunk {
        self.function.chunk()
    }

    pub fn upvalue_count(&self) -> usize {
        self.upvalues.len()
    }

    #[inline]
    pub fn get(&self, idx: usize) -> UpValue {
        self.upvalues[idx].clone()
    }
}

impl Trace<Object> for Closure {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        self.function.trace(tracer);
        self.upvalues
            .iter()
            .flat_map(|u| u.get())
            .for_each(|v| v.trace(tracer));
    }
}
