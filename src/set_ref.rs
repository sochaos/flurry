use crate::iter::*;
use crate::{HashSet, GuardRef, TryInsertError};
use crossbeam_epoch::Guard;
use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};
use std::hash::{BuildHasher, Hash};
use std::ops::Index;

/// A reference to a [`HashSet`], constructed with [`HashSet::pin`] or [`HashSet::with_guard`].
///
/// The current thread will be pinned for the duration of this reference.
/// Keep in mind that this prevents the collection of garbage generated by the set.
pub struct HashSetRef<'set, T, S = crate::DefaultHashBuilder> {
    set: &'set HashSet<T, S>,
    guard: GuardRef<'set>,
}

impl<K, V, S> HashSet<T, S> {
    /// Get a reference to this set with the current thread pinned.
    ///
    /// Keep in mind that for as long as you hold onto this, you are preventing the collection of
    /// garbage generated by the set.
    pub fn pin(&self) -> HashSetRef<'_, T, S> {
        HashSetRef {
            guard: GuardRef::Owned(self.guard()),
            set: &self,
        }
    }

    /// Get a reference to this set with the given guard.
    pub fn with_guard<'g>(&'g self, guard: &'g Guard) -> HashSetRef<'g, T, S> {
        HashSetRef {
            set: &self,
            guard: GuardRef::Ref(guard),
        }
    }
}

impl<T, S> HashSetRef<'_, T, S> {
    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'g K, &'g V)`.
    /// See also [`HashSet::iter`].
    pub fn iter(&self) -> Keys<'_, T, ()> {
        self.set.iter(&self.guard)
    }

    /// Returns the number of entries in the set.
    /// See also [`HashSet::len`].
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Returns `true` if the set is empty. Otherwise returns `false`.
    /// See also [`HashSet::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

impl<K, V, S> HashSetRef<'_, K, V, S>
where
    K: Clone,
{
    /// Tries to reserve capacity for at least additional more elements.
    /// See also [`HashSet::reserve`].
    pub fn reserve(&self, additional: usize) {
        self.set.reserve(additional, &self.guard)
    }

    /// Removes all entries from this set.
    /// See also [`HashSet::clear`].
    pub fn clear(&self) {
        self.set.clear(&self.guard);
    }
}

impl<K, V, S> HashSetRef<'_, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Tests if `key` is a key in this table.
    /// See also [`HashSet::contains_key`].
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.set.contains(value, &self.guard)
    }

    /// Returns the value to which `key` is setped.
    /// See also [`HashSet::get`].
    #[inline]
    pub fn get<'g, Q>(&'g self, value: &Q) -> Option<&'g V>
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.set.get(value, &self.guard)
    }
}

impl<T, S> HashSetRef<'_, T, S>
where
    T: 'static + Sync + Send + Clone + Hash + Eq,
    S: BuildHasher,
{
    /// Inserts a key-value pair into the set.
    ///
    /// See also [`HashSet::insert`].
    pub fn insert(&self, value: T) -> bool {
        self.set.insert(key, value, &self.guard)
    }

    /// If the value for the specified `key` is present, attempts to
    /// compute a new setping given the key and its current setped value.
    /// See also [`HashSet::compute_if_present`].
    pub fn compute_if_present<'g, Q, F>(&'g self, key: &Q, resetping_function: F) -> Option<&'g V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
        F: FnOnce(&K, &V) -> Option<V>,
    {
        self.set
            .compute_if_present(key, resetping_function, &self.guard)
    }

    /// Removes the key (and its corresponding value) from this set.
    /// See also [`HashSet::remove`].
    pub fn remove<'g, Q>(&'g self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.set.remove(value, &self.guard)
    }

    pub fn take<'g, Q>(&self, value: &Q) -> Option<&'g T>
        where
            T: Borrow<Q>,
            Q:?Sized + Hash + Eq
    {
        self.set.take(value, self.guard)
    }

    /// Retains only the elements specified by the predicate.
    /// See also [`HashSet::retain`].
    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        self.set.retain(f, &self.guard);
    }

    /// Retains only the elements specified by the predicate.
    /// See also [`HashSet::retain_force`].
    pub fn retain_force<F>(&self, f: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        self.set.retain_force(f, &self.guard);
    }
}

impl<'g, T, S> IntoIterator for &'g HashSetRef<'_, T, S> {
    type IntoIter = Keys<'g, T, ()>;
    type Item = &'g T;

    fn into_iter(self) -> Self::IntoIter {
        self.set.iter(&self.guard)
    }
}

impl<T, S> Debug for HashSetRef<'_, T, S>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self).finish()
    }
}

impl<T, S> Clone for HashSetRef<'_, T, S> {
    fn clone(&self) -> Self {
        Self {
            set: self.set.clone(),
            guard: self.guard.clone()
        }
    }
}

impl<T, S> PartialEq for HashSetRef<'_, T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.set == other.set
    }
}

impl<T, S> PartialEq<HashSet<T, S>> for HashSetRef<'_, T, S>
where
    K: Hash + Eq,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &HashSet<K, V, S>) -> bool {
        self.set.guarded_eq(&other, &self.guard, &other.guard())
    }
}

impl<T, S> PartialEq<HashSetRef<'_, T, S>> for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    fn eq(&self, other: &HashSetRef<'_, K, V, S>) -> bool {
        self.guarded_eq(&other.set, &self.guard(), &other.guard)
    }
}

impl<T, S> Eq for HashSetRef<'_, T, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
}
