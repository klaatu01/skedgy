use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Dep<T: ?Sized> {
    dep: Arc<T>,
}

impl<T> Dep<T> {
    pub fn new(state: T) -> Self {
        Dep {
            dep: Arc::new(state),
        }
    }

    pub fn inner(&self) -> &T {
        &self.dep
    }

    pub fn clone(&self) -> Self {
        Dep {
            dep: self.dep.clone(),
        }
    }
}

pub struct DependencyStore {
    pub states: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl DependencyStore {
    pub fn new() -> Self {
        DependencyStore {
            states: HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, state: T) {
        self.states
            .insert(TypeId::of::<T>(), Box::new(Dep::new(state)));
    }

    pub fn get<T: 'static>(&self) -> Option<Dep<T>> {
        self.states
            .get(&TypeId::of::<T>())
            .and_then(|boxed_any| boxed_any.downcast_ref::<Dep<T>>())
            .map(|state| state.clone())
    }
}
