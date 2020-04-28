pub enum Type {
    Float,
    Int,
    Bool,
    String,
    Nil
}

pub struct TypeInfo {
    mutable: bool,
    kind: Type
}

impl TypeInfo {
    pub fn new(kind: Type, mutable: bool) -> Self {
        TypeInfo {
            mutable,
            kind,
        }
    }
}