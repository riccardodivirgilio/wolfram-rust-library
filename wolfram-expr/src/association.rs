//! [`Association`][ref/Association]<sub>WL</sub> data type — `<|k -> v, ...|>`.
//!
//! Associations preserve insertion order and are wire-distinct from
//! `Function[Association, Rule[k, v], ...]` in WXF (token `'A'`).
//!
//! [ref/Association]: https://reference.wolfram.com/language/ref/Association.html

use crate::Expr;

/// Ordered key/value collection — Wolfram Language `<|...|>`.
///
/// Stored as a `Vec<(Expr, Expr)>` to preserve insertion order, mirroring WL's
/// `KeyValueMap`-style semantics. For value-based lookup use [`get`][Self::get];
/// for full traversal use [`iter`][Self::iter].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Association {
    pairs: Vec<(Expr, Expr)>,
}

impl Association {
    /// New empty association.
    pub fn new() -> Self {
        Association { pairs: Vec::new() }
    }

    /// New association from a vector of (key, value) pairs.
    pub fn from_pairs(pairs: Vec<(Expr, Expr)>) -> Self {
        Association { pairs }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the association is empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Append a key-value pair. If the key already exists, both pairs will be present
    /// (matching WL's behavior — duplicates are allowed but only the last is reachable
    /// via `Lookup`).
    pub fn insert(&mut self, key: Expr, value: Expr) {
        self.pairs.push((key, value));
    }

    /// Find the value associated with `key`. Returns the LAST inserted match (matches
    /// `Lookup[assoc, key]` semantics in WL).
    pub fn get(&self, key: &Expr) -> Option<&Expr> {
        self.pairs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Iterate over all (key, value) pairs in insertion order.
    pub fn iter(&self) -> std::slice::Iter<'_, (Expr, Expr)> {
        self.pairs.iter()
    }

    /// Borrow the underlying pair vector.
    pub fn as_pairs(&self) -> &[(Expr, Expr)] {
        &self.pairs
    }

    /// Consume into a vector of pairs.
    pub fn into_pairs(self) -> Vec<(Expr, Expr)> {
        self.pairs
    }
}

impl<'a> IntoIterator for &'a Association {
    type Item = &'a (Expr, Expr);
    type IntoIter = std::slice::Iter<'a, (Expr, Expr)>;
    fn into_iter(self) -> Self::IntoIter {
        self.pairs.iter()
    }
}

impl IntoIterator for Association {
    type Item = (Expr, Expr);
    type IntoIter = std::vec::IntoIter<(Expr, Expr)>;
    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl From<Vec<(Expr, Expr)>> for Association {
    fn from(pairs: Vec<(Expr, Expr)>) -> Self {
        Association { pairs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Symbol;

    #[test]
    fn empty() {
        let a = Association::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn insertion_order_preserved() {
        let mut a = Association::new();
        a.insert(Expr::from("first"), Expr::from(1));
        a.insert(Expr::from("second"), Expr::from(2));
        a.insert(Expr::from("third"), Expr::from(3));

        let keys: Vec<&str> = a
            .iter()
            .filter_map(|(k, _)| k.try_as_str())
            .collect();
        assert_eq!(keys, ["first", "second", "third"]);
    }

    #[test]
    fn lookup_returns_last_match() {
        let mut a = Association::new();
        a.insert(Expr::from("key"), Expr::from(1));
        a.insert(Expr::from("key"), Expr::from(2));
        assert_eq!(a.get(&Expr::from("key")), Some(&Expr::from(2)));
    }

    #[test]
    fn lookup_missing() {
        let a = Association::from_pairs(vec![(Expr::from("k"), Expr::from(1))]);
        assert_eq!(a.get(&Expr::from("missing")), None);
    }

    #[test]
    fn lookup_by_symbol() {
        let key = Expr::symbol(Symbol::new("Global`x"));
        let mut a = Association::new();
        a.insert(key.clone(), Expr::from(42));
        assert_eq!(a.get(&key), Some(&Expr::from(42)));
    }
}
