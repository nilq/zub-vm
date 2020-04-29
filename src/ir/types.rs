#[derive(Clone)]
pub enum Type {
    Float,
    Int,
    Bool,
    String,
    Nil
}

#[derive(Clone)]
pub struct TypeInfo {
    mutable: bool,
    kind: Option<Type>
}

impl TypeInfo {
    pub fn new(kind: Type, mutable: bool) -> Self {
        TypeInfo {
            mutable,
            kind: Some(kind),
        }
    }

    pub fn none(mutable: bool) -> Self {
        TypeInfo {
            mutable,
            kind: None,
        }
    }
}