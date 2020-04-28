use super::super::gc::{ *, tag::*, trace::* };
use super::*;

use std::fmt::{Debug, Display};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Value {
    handle: TaggedHandle<Object>,
}

#[derive(Debug, Clone)]
pub enum Variant {
    Float(f64),
    True,
    False,
    Nil,
    Obj(Handle<Object>),
}

const TAG_TRUE:  u8 = 0x01;
const TAG_FALSE: u8 = 0x02;
const TAG_NIL:   u8 = 0x03;

impl Value {
    #[inline]
    pub unsafe fn from_raw(raw: u64) -> Self {
        Value {
            handle: TaggedHandle::from_raw(raw),
        }
    }

    pub fn to_raw(self) -> u64 {
        self.handle.to_raw()
    }

    #[inline]
    pub fn as_float(&self) -> f64 {
        if let Variant::Float(f) = self.decode() {
            return f
        }

        panic!("non-float")
    }

    #[inline]
    pub fn decode(&self) -> Variant {
        use self::Tag::*;

        match self.handle.clone().decode() {
            Float(n) => Variant::Float(n),
            Handle(n) => Variant::Obj(n),
            Tag(t) if t == TAG_TRUE  => Variant::True,
            Tag(t) if t == TAG_FALSE => Variant::False,
            Tag(t) if t == TAG_NIL   => Variant::Nil,
            Tag(t) => panic!("Unknown tag: {}", t)
        }
    }

    #[inline]
    pub fn as_object<'a>(&self) -> Option<Handle<Object>> {
        match self.decode() {
            Variant::Obj(o) => Some(o),
            _ => None,
        }
    }

    pub fn with_heap<'h>(&self, heap: &'h Heap<Object>) -> WithHeap<'h, Self> {
        WithHeap::new(heap, *self)
    }

    pub fn float(float: f64) -> Self {
        Value {
            handle: TaggedHandle::from_float(float),
        }
    }

    pub fn truelit() -> Self {
        Value {
            handle: TaggedHandle::from_tag(TAG_TRUE),
        }
    }

    pub fn falselit() -> Self {
        Value {
            handle: TaggedHandle::from_tag(TAG_FALSE),
        }
    }

    pub fn truthy(&self) -> bool {
        match self.decode() {
            Variant::False | Variant::Nil => false,
            _ => true,
        }
    }

    pub fn nil() -> Self {
        Value {
            handle: TaggedHandle::from_tag(TAG_NIL),
        }
    }

    pub fn object(handle: Handle<Object>) -> Self {
        Value {
            handle: TaggedHandle::from_handle(handle)
        }
    }
}

impl Trace<Object> for Value {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        if let Variant::Obj(obj) = self.decode() {
            obj.trace(tracer);
        }
    }
}

impl From<Handle<Object>> for Value {
    fn from(handle: Handle<Object>) -> Self {
        Value::object(handle)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self.decode() {
            Variant::Nil => write!(f, "nil"),
            Variant::False => write!(f, "false"),
            Variant::True => write!(f, "true"),
            Variant::Float(n) => write!(f, "{:?}", n),
            Variant::Obj(o) => write!(f, "{:?}", o),
        }
    }
}

impl Into<Value> for f64 {
    fn into(self) -> Value {
        Value::float(self)
    }
}

impl Into<Value> for bool {
    fn into(self) -> Value {
        if self {
            Value::truelit()
        } else {
            Value::falselit()
        }
    }
}

pub struct WithHeap<'h, T> {
    pub heap: &'h Heap<Object>,
    pub item: T,
}

impl<'h, T> WithHeap<'h, T> {
    pub fn new(heap: &'h Heap<Object>, item: T) -> WithHeap<'h, T> {
        WithHeap { heap, item }
    }

    pub fn with<U>(&self, item: U) -> WithHeap<U> {
        WithHeap { heap: self.heap, item }
    }
}

impl<'h> Display for WithHeap<'h, Value> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self.item.decode() {
            Variant::Nil => write!(f, "nil"),
            Variant::False => write!(f, "false"),
            Variant::True => write!(f, "true"),
            Variant::Float(n) => write!(f, "{}", n),
            Variant::Obj(o) => {
                let o = self.heap.get(o).ok_or(::std::fmt::Error)?;
                write!(f, "{}", self.with(o))
            },
        }
    }
}
