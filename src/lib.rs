#![warn(missing_docs)]

//! A "cycle-safe" topological sort for a set of nodes with dependencies in Rust.
//! Basically, it allows sorting a list by its dependencies while checking for
//! cycles in the graph. If a cycle is detected, a `CycleError` is returned from the
//! iterator.
//!
//! ## Usage
//!
//! ```toml
//! [dependencies]
//! topo_sort = "0.1"
//! ```
//!
//! A basic example:
//!
//! ```rust
//! use topo_sort::TopoSort;
//!
//! let mut topo_sort = TopoSort::with_capacity(5);
//! topo_sort.insert("C", vec!["A", "B"]); // read: "C" depends on "A" and "B"
//! topo_sort.insert("E", vec!["B", "C"]);
//! topo_sort.insert("A", vec![]);
//! topo_sort.insert("D", vec!["A", "C", "E"]);
//! topo_sort.insert("B", vec!["A"]);
//!
//! assert_eq!(
//!     vec!["A", "B", "C", "E", "D"],
//!     topo_sort.try_owned_vec().unwrap()
//! );
//! ```
//!
//! ...or using iteration:
//!
//! ```rust
//! use topo_sort::TopoSort;
//!
//! let mut topo_sort = TopoSort::with_capacity(5);
//! topo_sort.insert("C", vec!["A", "B"]);
//! topo_sort.insert("E", vec!["B", "C"]);
//! topo_sort.insert("A", vec![]);
//! topo_sort.insert("D", vec!["A", "C", "E"]);
//! topo_sort.insert("B", vec!["A"]);
//!
//! let mut nodes = Vec::with_capacity(5);
//! for node in &topo_sort {
//!     // We check for cycle errors before usage
//!     match node {
//!         Ok((node, _)) => nodes.push(*node),
//!         Err(_) => panic!("Unexpected cycle!"),
//!     }
//! }
//!
//! assert_eq!(vec!["A", "B", "C", "E", "D"], nodes);
//! ```

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Index;
use std::{error, fmt};

// *** Error ***

/// An error type returned by the iterator when a cycle is detected in the dependency graph
#[derive(Clone, Copy, fmt::Debug, PartialEq)]
pub struct CycleError;

impl fmt::Display for CycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for CycleError {}

// *** TopoSort ***

/// TopoSort is used as a collection to map nodes to their dependencies. The actual sort is "lazy" and is performed during iteration.
#[derive(Clone, Default)]
pub struct TopoSort<T> {
    // Dependent -> Dependencies
    node_depends: HashMap<T, HashSet<T>>,
}

impl<T> TopoSort<T>
where
    T: Eq + Hash,
{
    /// Initialize a new struct with zero capacity. It will not allocate until the first insertion
    #[inline]
    pub fn new() -> Self {
        TopoSort {
            node_depends: HashMap::new(),
        }
    }

    /// Initialize a new struct from a map. The key represents the node to be sorted and the set is its dependencies
    #[inline]
    pub fn from_map(nodes: HashMap<T, HashSet<T>>) -> Self {
        TopoSort {
            node_depends: nodes,
        }
    }

    /// Initialize an empty struct with a given capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        TopoSort {
            node_depends: HashMap::with_capacity(capacity),
        }
    }

    /// Insert into this struct with the given node and a slice of its dependencies
    pub fn insert_from_slice(&mut self, node: T, slice: &[T])
    where
        T: Clone,
    {
        self.node_depends
            .insert(node, HashSet::from_iter(slice.to_vec()));
    }

    /// Insert into this struct with the given node and a set of its dependencies
    #[inline]
    pub fn insert_from_set(&mut self, node: T, depends: HashSet<T>) {
        self.node_depends.insert(node, depends);
    }

    /// Insert into this struct with the given node and an iterator of its dependencies
    #[inline]
    pub fn insert<I: IntoIterator<Item = T>>(&mut self, node: T, i: I) {
        self.node_depends.insert(node, i.into_iter().collect());
    }

    /// Start the sort process and return an iterator of the results
    #[inline]
    pub fn nodes(&self) -> TopoSortNodeIter<'_, T> {
        TopoSortNodeIter::new(&self.node_depends)
    }

    /// Start the sort process and return an iterator of the results and a set of its dependents
    #[inline]
    pub fn iter(&self) -> TopoSortIter<'_, T> {
        TopoSortIter::new(&self.node_depends)
    }

    /// Sort and return a vector (with borrowed nodes) of the results. If a cycle is detected,
    /// an error is returned instead
    #[inline]
    pub fn try_vec(&self) -> Result<Vec<&T>, CycleError> {
        self.nodes().collect()
    }

    /// Sort and return a vector (with owned/cloned nodes) of the results. If a cycle is detected,
    /// an error is returned instead
    pub fn try_owned_vec(&self) -> Result<Vec<T>, CycleError>
    where
        T: Clone,
    {
        self.nodes()
            .map(|result| result.map(|node| node.clone()))
            .collect()
    }

    /// Returns true if there aren't any nodes added otherwise false
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.node_depends.is_empty()
    }

    /// Returns the number of nodes added to the collection
    #[inline]
    pub fn len(&self) -> usize {
        self.node_depends.len()
    }

    /// Returns the dependency set of a node (as inserted), if found, else None
    #[inline]
    pub fn get(&self, node: &T) -> Option<&HashSet<T>> {
        self.node_depends.get(node)
    }
}

impl<T> Index<&T> for TopoSort<T>
where
    T: Eq + Hash,
{
    type Output = HashSet<T>;

    #[inline]
    fn index(&self, index: &T) -> &Self::Output {
        self.node_depends.index(index)
    }
}

impl<T> IntoIterator for TopoSort<T>
where
    T: Eq + Hash,
{
    type Item = Result<(T, HashSet<T>), CycleError>;
    type IntoIter = IntoTopoSortIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoTopoSortIter::new(self.node_depends)
    }
}

impl<'d, T> IntoIterator for &'d TopoSort<T>
where
    T: Eq + Hash,
{
    type Item = Result<(&'d T, &'d HashSet<T>), CycleError>;
    type IntoIter = TopoSortIter<'d, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// *** InnerIter ***

// Dependency -> (Dependents, Edge Count)
type Nodes<T> = HashMap<*const T, (HashSet<*const T>, u32)>;

struct InnerIter<T> {
    nodes: Nodes<T>,
    no_edges: Vec<*const T>,
}

impl<T> InnerIter<T>
where
    T: Eq + Hash,
{
    fn new(node_depends: &HashMap<T, HashSet<T>>) -> Self {
        let nodes = Self::make_nodes(node_depends);
        let no_edges = Self::make_no_edges(&nodes);
        InnerIter { nodes, no_edges }
    }

    fn make_nodes(node_depends: &HashMap<T, HashSet<T>>) -> Nodes<T> {
        // Avoids borrow issues in closure
        let len = node_depends.len();
        let mut nodes: Nodes<T> = HashMap::with_capacity(len);
        // Assume no dependents for now (TODO: How to pick a good # here to minimize reallocation but doesn't go crazy?)
        let new_entry_fn = || (HashSet::new(), 0);

        // We need to ensure that every `*const T` is based off `&T` from the key in `node_depends`
        // NOTE: This looks odd but remember that `Eq` and `Hash` are off the value of `T`, not it's address
        // so we need to lookup the address even though it looks like an identity op... it isn't
        let lookup: HashMap<_, _> = node_depends.keys().map(|key| (key, key)).collect();

        for (dependent, dependencies) in node_depends {
            // Don't overwrite if we have it already (from a dependency below), but otherwise ensure every node is added
            nodes.entry(dependent).or_insert_with(new_entry_fn);

            for dependency in dependencies {
                // Filter any self references
                if dependent != dependency {
                    // We need to swap to the `&T` based on `dependent` before going further
                    // `dependency` must be in `node_depends` to qualify for continued processing
                    if let Some(&dependency) = lookup.get(dependency) {
                        // Each dependent tracks the # of dependencies
                        // NOTE: The `or_insert_with` will never be executed, but I just liked it better than casting to `*const T` with `get_mut`
                        let dependent_entry = nodes.entry(dependent).or_insert_with(new_entry_fn);
                        dependent_entry.1 += 1;

                        // Each dependency tracks all it's dependents
                        let dependency_entry = nodes.entry(dependency).or_insert_with(new_entry_fn);
                        dependency_entry.0.insert(dependent);
                    }
                }
            }
        }

        nodes
    }

    fn make_no_edges(nodes: &Nodes<T>) -> Vec<*const T> {
        // Find first batch of ready nodes (TODO: move into loop so we can set capacity? What capacity to set?)
        nodes
            .iter()
            .filter(|(_, (_, edges))| *edges == 0)
            .map(|(&node, _)| node)
            .collect()
    }

    fn next(&mut self) -> Option<Result<*const T, CycleError>> {
        match self.no_edges.pop() {
            Some(node) => {
                // NOTE: Unwrap() should be safe - we know it was in there since it came from there
                // We are done with this node - remove entirely
                let (dependents, _) = &self
                    .nodes
                    .remove(&node)
                    .expect("node not in `nodes` on remove");

                // Decrement the edge count of all nodes that depend on this one and add them
                // to no_edges when they hit zero
                for &dependent in dependents {
                    // NOTE: Unwrap() should be safe - we know it was in there from init
                    let (_, edges) = self
                        .nodes
                        .get_mut(&dependent)
                        .expect("dependent not found in `nodes`");
                    *edges -= 1;
                    if *edges == 0 {
                        self.no_edges.push(dependent);
                    }
                }

                Some(Ok(node))
            }
            None if self.nodes.is_empty() => None,
            None => {
                self.nodes.clear();
                Some(Err(CycleError))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.nodes.len();
        (len, Some(len))
    }
}

// *** IntoTopoSortIter ***

/// Consuming/owning iterator over the final node and dependent set of the topological sort
pub struct IntoTopoSortIter<T> {
    inner: InnerIter<T>,

    // Dependent -> Dependencies
    node_depends: HashMap<T, HashSet<T>>,
}

impl<T> IntoTopoSortIter<T>
where
    T: Eq + Hash,
{
    fn new(node_depends: HashMap<T, HashSet<T>>) -> Self {
        IntoTopoSortIter {
            inner: InnerIter::new(&node_depends),
            node_depends,
        }
    }
}

impl<T> Iterator for IntoTopoSortIter<T>
where
    T: Eq + Hash,
{
    type Item = Result<(T, HashSet<T>), CycleError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map(|node| unsafe {
                // NOTE: This depends on the HashMap NOT shrinking on remove - if this ever changes this
                // will likely break as the addresses of the keys will change
                self.node_depends
                    .remove_entry(&*node)
                    .expect("node not in `node_depends` on remove")
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// *** TopoSortIter ***

/// Iterator over the final node and dependent set of the topological sort
pub struct TopoSortIter<'d, T> {
    inner: InnerIter<T>,

    // Dependent -> Dependencies
    node_depends: &'d HashMap<T, HashSet<T>>,
}

impl<'d, T> TopoSortIter<'d, T>
where
    T: Eq + Hash,
{
    fn new(node_depends: &'d HashMap<T, HashSet<T>>) -> Self {
        TopoSortIter {
            inner: InnerIter::new(node_depends),
            node_depends,
        }
    }
}

impl<'d, T> Iterator for TopoSortIter<'d, T>
where
    T: Eq + Hash,
{
    type Item = Result<(&'d T, &'d HashSet<T>), CycleError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map(|node| {
                // Safe: We ensure every node is always added first thing in the loop in 'new'
                unsafe { (&*node, &self.node_depends[&*node]) }
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// *** TopoSortNodeIter ***

/// Iterator over the final node only of the topological sort
pub struct TopoSortNodeIter<'d, T>(TopoSortIter<'d, T>);

impl<'d, T> TopoSortNodeIter<'d, T>
where
    T: Eq + Hash,
{
    #[inline]
    fn new(node_depends: &'d HashMap<T, HashSet<T>>) -> Self {
        TopoSortNodeIter(TopoSortIter::new(node_depends))
    }
}

impl<'d, T> Iterator for TopoSortNodeIter<'d, T>
where
    T: Eq + Hash,
{
    type Item = Result<&'d T, CycleError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|result| result.map(|(node, _)| node))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

// *** Tests ***

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{CycleError, TopoSort};

    #[test]
    fn test_termination() {
        let mut topo_sort = TopoSort::with_capacity(4);
        topo_sort.insert(1, vec![2]);
        topo_sort.insert(2, vec![1]); // cycle
        topo_sort.insert(3, vec![4]);
        topo_sort.insert(4, vec![]);

        let v: Vec<Result<_, _>> = topo_sort.nodes().collect();
        assert_eq!(vec![Ok(&4), Ok(&3), Err(CycleError)], v);
    }

    #[test]
    fn test_direct_cycle() {
        let mut topo_sort = TopoSort::with_capacity(2);
        topo_sort.insert(1, vec![2]);
        topo_sort.insert(2, vec![1]); // cycle

        assert!(topo_sort.try_vec().is_err())
    }

    #[test]
    fn test_indirect_cycle() {
        let mut topo_sort = TopoSort::with_capacity(3);
        topo_sort.insert(1, vec![2, 3]);
        topo_sort.insert(2, vec![3]);
        topo_sort.insert(3, vec![1]); // cycle

        assert!(topo_sort.try_vec().is_err())
    }

    #[test]
    fn test_good() {
        let mut topo_sort = TopoSort::with_capacity(5);
        topo_sort.insert("C", vec!["A", "B"]);
        topo_sort.insert("E", vec!["B", "C"]);
        topo_sort.insert("A", vec![]);
        topo_sort.insert("D", vec!["A", "C", "E"]);
        topo_sort.insert("B", vec!["A"]);

        assert_eq!(
            vec!["A", "B", "C", "E", "D"],
            topo_sort.try_owned_vec().unwrap()
        );
    }

    #[test]
    fn test_good_with_no_depends() {
        let mut topo_sort = TopoSort::with_capacity(1);
        topo_sort.insert("C", vec![]);

        assert_eq!(vec!["C"], topo_sort.try_owned_vec().unwrap());
    }

    #[test]
    fn test_good_with_excess_depends() {
        let mut topo_sort = TopoSort::with_capacity(5);
        topo_sort.insert("C", vec!["F", "A", "B", "F"]); // There is no 'F' - two of them
        topo_sort.insert("E", vec!["C", "B", "C"]); // Double "C" dependency
        topo_sort.insert("A", vec!["A", "G"]); // Self dependency + there is no 'G'
        topo_sort.insert("D", vec!["A", "C", "E"]);
        topo_sort.insert("B", vec!["B", "A"]); // Self dependency

        assert_eq!(
            vec!["A", "B", "C", "E", "D"],
            topo_sort.try_owned_vec().unwrap()
        );
    }

    #[test]
    fn test_loop() {
        let mut topo_sort = TopoSort::with_capacity(5);
        topo_sort.insert("C", vec!["A", "B"]);
        topo_sort.insert("E", vec!["B", "C"]);
        topo_sort.insert("A", vec![]);
        topo_sort.insert("D", vec!["A", "C", "E"]);
        topo_sort.insert("B", vec!["A"]);

        let mut nodes = Vec::with_capacity(5);
        for node in &topo_sort {
            // Must check for cycle errors before usage
            match node {
                Ok((node, _)) => nodes.push(*node),
                Err(_) => panic!("Unexpected cycle!"),
            }
        }

        assert_eq!(vec!["A", "B", "C", "E", "D"], nodes);
    }

    #[test]
    fn test_consuming_iter() {
        let mut topo_sort = TopoSort::with_capacity(5);
        topo_sort.insert("C", vec!["A", "B"]);
        topo_sort.insert("E", vec!["B", "C"]);
        topo_sort.insert("A", vec![]);
        topo_sort.insert("D", vec!["A", "C", "E"]);
        topo_sort.insert("B", vec!["A"]);

        let mut nodes = Vec::with_capacity(5);
        for node in topo_sort {
            // Must check for cycle errors before usage
            match node {
                Ok((node, _)) => nodes.push(node),
                Err(_) => panic!("Unexpected cycle!"),
            }
        }

        assert_eq!(vec!["A", "B", "C", "E", "D"], nodes);
    }

    #[test]
    fn test_misc() {
        let mut topo_sort = TopoSort::new();
        assert!(topo_sort.is_empty());

        topo_sort.insert_from_slice("A", &["B"]);
        assert_eq!(1, topo_sort.len());

        let set = HashSet::from_iter(vec!["B", "D"]);
        topo_sort.insert_from_set("C", set.clone());

        assert_eq!(set, *topo_sort.get(&"C").unwrap());
        assert_eq!(set, topo_sort[&"C"]);

        assert_eq!(None, topo_sort.get(&"D"));
    }
}
