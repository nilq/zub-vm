use std::{sync::Mutex, rc::Rc};

use super::{Heap, Value, VM, Object, WithHeap, Handle};

pub struct CallContext<'vm> {
    pub vm: &'vm mut VM,
    frame_start: usize,
}

impl<'vm> CallContext<'vm> {
    pub fn new(vm: &'vm mut VM, frame_start: usize) -> Self { Self { vm, frame_start } }

    pub fn get_arg(&mut self, index: usize) -> Value {
        let args = &self.vm.stack[self.frame_start..];
        args[index]
    }

    pub fn get_arg_with_heap(&mut self, index: usize) -> WithHeap<'_, Value> {
        self.get_arg(index).with_heap(&self.vm.heap)
    }

    pub fn with_heap(&mut self, value: Value) -> WithHeap<'_, Value> {
        value.with_heap(&self.vm.heap)
    }

    pub fn call(&mut self, function: Handle<Object>, args: Vec<Value>) -> Value {
        let vm = &mut self.vm;
        vm.internal_call(function, args)
    }
}
