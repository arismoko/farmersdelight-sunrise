//! A reload-owned table: built completely, then swapped in one step (PORT_PLAN §6.5).
use std::sync::{Arc, RwLock};

pub struct Published<T> {
    table: RwLock<Arc<[T]>>,
}

impl<T> Default for Published<T> {
    fn default() -> Self {
        Self {
            table: RwLock::new(Arc::from(Vec::new())),
        }
    }
}

impl<T> Published<T> {
    /// Replace the whole table (also when the new one is empty).
    pub fn publish(&self, next: Vec<T>) {
        *self.table.write().unwrap_or_else(|e| e.into_inner()) = next.into();
    }

    pub fn clear(&self) {
        self.publish(Vec::new());
    }

    /// A snapshot: later publications never change a table already handed out.
    pub fn snapshot(&self) -> Arc<[T]> {
        self.table.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_replaces_and_snapshots_are_stable() {
        let p = Published::default();
        p.publish(vec![1, 2]);
        let old = p.snapshot();
        p.publish(vec![3]);
        assert_eq!(&*old, &[1, 2]);
        assert_eq!(&*p.snapshot(), &[3]);
        p.clear();
        assert!(p.snapshot().is_empty());
    }
}
