pub mod trace;
pub mod tag;

use std::{
    cmp::{PartialEq, Eq},
    rc::Rc,
    hash::{Hash, Hasher},
};
use hashbrown::{HashMap, HashSet};
use trace::*;

type Generation = usize;

#[derive(Clone)]
pub struct Heap<T> {
    last_sweep: usize,
    object_sweeps: HashMap<Handle<T>, usize>,
    obj_counter: Generation,
    objects: HashSet<Handle<T>>,
    rooted: HashMap<Handle<T>, Rc<()>>,
}

impl<T> Default for Heap<T> {
    fn default() -> Self {
        Self {
            last_sweep: 0,
            object_sweeps: HashMap::default(),
            obj_counter: 0,
            objects: HashSet::default(),
            rooted: HashMap::default(),
        }
    }
}

impl<T: Trace<T>> Heap<T> {
    /// Create an empty heap.
    pub fn new() -> Self {
        Self::default()
    }

    fn new_generation(&mut self) -> Generation {
        self.obj_counter += 1;
        self.obj_counter
    }

    /// Adds a new object to this heap that will be cleared upon the next garbage collection, if
    /// not attached to the object tree.
    pub fn insert_temp(&mut self, object: T) -> Handle<T> {
        let ptr = Box::into_raw(Box::new(object));

        let gen = self.new_generation();
        let handle = Handle { gen, ptr };
        self.objects.insert(handle);

        handle
    }

    /// Adds a new object to this heap that will not be cleared by garbage collection until all
    /// rooted handles have been dropped.
    pub fn insert(&mut self, object: T) -> Rooted<T> {
        let handle = self.insert_temp(object);

        let rc = Rc::new(());
        self.rooted.insert(handle, rc.clone());

        Rooted {
            rc,
            handle,
        }
    }

    /// Upgrade a handle (that will be cleared by the garbage collector) into a rooted handle (that
    /// will not).
    pub fn make_rooted(&mut self, handle: impl AsRef<Handle<T>>) -> Rooted<T> {
        let handle = handle.as_ref();
        debug_assert!(self.contains(handle));

        Rooted {
            rc: self.rooted
                .entry(*handle)
                .or_insert_with(|| Rc::new(()))
                .clone(),
            handle: *handle,
        }
    }

    /// Count the number of heap-allocated objects in this heap
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Return true if the heap contains the specified handle
    pub fn contains(&self, handle: impl AsRef<Handle<T>>) -> bool {
        let handle = handle.as_ref();
        self.objects.contains(&handle)
    }

    /// Get a reference to a heap object if it exists on this heap.
    pub fn get(&self, handle: impl AsRef<Handle<T>>) -> Option<&T> {
        let handle = handle.as_ref();
        if self.contains(handle) {
            Some(unsafe { &*handle.ptr })
        } else {
            None
        }
    }

    /// Get a reference to a heap object without checking whether it is still alive or that it
    /// belongs to this heap.
    ///
    /// If either invariant is not upheld, calling this function results in undefined
    /// behaviour.
    pub unsafe fn get_unchecked(&self, handle: impl AsRef<Handle<T>>) -> &T {
        let handle = handle.as_ref();
        debug_assert!(self.contains(handle));
        &*handle.ptr
    }

    /// Get a mutable reference to a heap object
    pub fn get_mut(&mut self, handle: impl AsRef<Handle<T>>) -> Option<&mut T> {
        let handle = handle.as_ref();
        if self.contains(handle) {
            Some(unsafe { &mut *handle.ptr })
        } else {
            None
        }
    }

    /// Get a mutable reference to a heap object without first checking that it is still alive or
    /// that it belongs to this heap.
    ///
    /// If either invariant is not upheld, calling this function results in undefined
    /// behaviour. Provided they are upheld, this function provides zero-cost access.
    pub fn get_mut_unchecked(&mut self, handle: impl AsRef<Handle<T>>) -> &mut T {
        let handle = handle.as_ref();
        debug_assert!(self.contains(handle));
        unsafe { &mut *handle.ptr }
    }

    pub fn clean_excluding(&mut self, excluding: impl IntoIterator<Item=Handle<T>>) {
        let new_sweep = self.last_sweep + 1;
        let mut tracer = Tracer {
            new_sweep,
            object_sweeps: &mut self.object_sweeps,
            objects: &self.objects,
        };

        // Mark
        self.rooted
            .retain(|handle, rc| {
                if Rc::strong_count(rc) > 1 {
                    tracer.mark(*handle);
                    unsafe { (&*handle.ptr).trace(&mut tracer); }
                    true
                } else {
                    false
                }
            });
        let objects = &self.objects;
        excluding
            .into_iter()
            .filter(|handle| objects.contains(&handle))
            .for_each(|handle| {
                tracer.mark(handle);
                unsafe { (&*handle.ptr).trace(&mut tracer); }
            });

        // Sweep
        let object_sweeps = &mut self.object_sweeps;
        self.objects
            .retain(|handle| {
                if object_sweeps
                    .get(handle)
                    .map(|sweep| *sweep == new_sweep)
                    .unwrap_or(false)
                {
                    true
                } else {
                    object_sweeps.remove(handle);
                    drop(unsafe { Box::from_raw(handle.ptr) });
                    false
                }
            });

        self.last_sweep = new_sweep;
    }

    /// Clean orphaned objects from the heap.
    pub fn clean(&mut self) {
        self.clean_excluding(std::iter::empty());
    }
}

impl<T> Drop for Heap<T> {
    fn drop(&mut self) {
        for handle in &self.objects {
            drop(unsafe { Box::from_raw(handle.ptr) });
        }
    }
}

#[derive(Debug)]
pub struct Handle<T> {
    gen: Generation,
    ptr: *mut T,
}

impl<T> Handle<T> {
    pub unsafe fn get_unchecked(&self) -> &T {
        &*self.ptr
    }

    pub unsafe fn get_mut_unchecked(&self) -> &mut T {
        &mut *self.ptr
    }
}

impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self { gen: self.gen, ptr: self.ptr }
    }
}

impl<T> PartialEq<Self> for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.gen == other.gen && self.ptr == other.ptr
    }
}
impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.gen.hash(state);
        self.ptr.hash(state);
    }
}

impl<T> AsRef<Handle<T>> for Handle<T> {
    fn as_ref(&self) -> &Handle<T> {
        self
    }
}

impl<T> From<Rooted<T>> for Handle<T> {
    fn from(rooted: Rooted<T>) -> Self {
        rooted.handle
    }
}

#[derive(Debug)]
pub struct Rooted<T> {
    rc: Rc<()>,
    handle: Handle<T>,
}

impl<T> Clone for Rooted<T> {
    fn clone(&self) -> Self {
        Self {
            rc: self.rc.clone(),
            handle: self.handle,
        }
    }
}

impl<T> AsRef<Handle<T>> for Rooted<T> {
    fn as_ref(&self) -> &Handle<T> {
        &self.handle
    }
}

impl<T> Rooted<T> {
    pub fn into_handle(self) -> Handle<T> {
        self.handle
    }

    pub fn handle(&self) -> Handle<T> {
        self.handle
    }
}