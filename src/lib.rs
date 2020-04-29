pub mod vm;
pub mod ir;
pub mod compiler;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::vm::*;
        use super::ir::*;

        let mut builder = IrBuilder::new();

        let value = builder.number(42.0);
        builder.bind_local("foo", value, 0, 0);

        let mut vm = VM::new();

        vm.exec(&builder.build())
    }
}
