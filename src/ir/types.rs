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
    kind: Option<Type>
}

impl TypeInfo {
    pub fn new(kind: Type) -> Self {
        TypeInfo {
            kind: Some(kind),
        }
    }

    pub fn nil() -> Self {
        TypeInfo {
            kind: None,
        }
    }
}