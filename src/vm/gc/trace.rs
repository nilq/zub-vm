use super::*;


pub trait Trace<T: Trace<T>> {
    fn trace(&self, tracer: &mut Tracer<T>);
}

pub struct Tracer<'a, T: Trace<T>> {
    pub(crate) new_sweep: usize,
    pub(crate) object_sweeps: &'a mut HashMap<Handle<T>, usize>,
    pub(crate) objects: &'a HashSet<Handle<T>>,
}

impl<'a, T: Trace<T>> Tracer<'a, T> {
    pub(crate) fn mark(&mut self, handle: Handle<T>) {
        let sweep = self.object_sweeps
            .entry(handle)
            .or_insert(self.new_sweep - 1);
        if *sweep != self.new_sweep && self.objects.contains(&handle) {
            *sweep = self.new_sweep;
            unsafe { (&*handle.ptr).trace(self); }
        }
    }
}

impl<O: Trace<O>> Trace<O> for Handle<O> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        tracer.mark(*self);
    }
}

impl<O: Trace<O>> Trace<O> for Rooted<O> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.handle().trace(tracer);
    }
}

use std::collections::{
    HashMap as StdHashMap,
    VecDeque,
    LinkedList,
};

impl<O: Trace<O>, T: Trace<O>> Trace<O> for [T] {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.iter().for_each(|object| object.trace(tracer));
    }
}

impl<O: Trace<O>, T: Trace<O>> Trace<O> for VecDeque<T> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.iter().for_each(|object| object.trace(tracer));
    }
}

impl<O: Trace<O>, T: Trace<O>> Trace<O> for LinkedList<T> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.iter().for_each(|object| object.trace(tracer));
    }
}

impl<O: Trace<O>, K, V: Trace<O>> Trace<O> for StdHashMap<K, V> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.values().for_each(|object| object.trace(tracer));
    }
}

impl<O: Trace<O>, T: Trace<O>> Trace<O> for HashSet<T> {
    fn trace(&self, tracer: &mut Tracer<O>) {
        self.iter().for_each(|object| object.trace(tracer));
    }
}
