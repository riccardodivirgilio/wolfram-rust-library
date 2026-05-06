//! [`Association`][ref/Association]<sub>WL</sub> data type — `<|k -> v, ...|>`.
//!
//! Wire-distinct from `Function[Association, Rule[k, v], ...]` in WXF (token `'A'`).
//!
//! Implementation: a [`BTreeMap`] keyed by [`Expr`] for O(log n) lookup. Each value
//! tracks whether the entry is a `Rule` (`->`, immediate) or `RuleDelayed` (`:>`,
//! held). Iteration order is by key (sorted), not insertion order.
//!
//! [ref/Association]: https://reference.wolfram.com/language/ref/Association.html

use std::collections::BTreeMap;
use std::iter::FromIterator;

use crate::Expr;

/// Single association entry — value plus a flag indicating Rule vs RuleDelayed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleEntry {
    /// The right-hand side expression.
    pub value: Expr,
    /// `false` for `Rule` (`->`, immediate), `true` for `RuleDelayed` (`:>`, held).
    pub delayed: bool,
}

impl RuleEntry {
    /// Construct a `Rule` (`->`, immediate) entry.
    pub fn rule(value: Expr) -> Self {
        RuleEntry {
            value,
            delayed: false,
        }
    }

    /// Construct a `RuleDelayed` (`:>`, held) entry.
    pub fn rule_delayed(value: Expr) -> Self {
        RuleEntry {
            value,
            delayed: true,
        }
    }
}

/// Wolfram Language `<|...|>` — keyed expression collection with fast lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Association {
    map: BTreeMap<Expr, RuleEntry>,
}

impl Association {
    /// New empty association.
    pub fn new() -> Self {
        Association {
            map: BTreeMap::new(),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the association is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Insert a `Rule` (`->`, immediate). Replaces any existing entry with the same key.
    pub fn insert(&mut self, key: Expr, value: Expr) -> Option<RuleEntry> {
        self.map.insert(key, RuleEntry::rule(value))
    }

    /// Insert a `RuleDelayed` (`:>`, held). Replaces any existing entry with the same key.
    pub fn insert_delayed(&mut self, key: Expr, value: Expr) -> Option<RuleEntry> {
        self.map.insert(key, RuleEntry::rule_delayed(value))
    }

    /// Insert a raw [`RuleEntry`].
    pub fn insert_entry(&mut self, key: Expr, entry: RuleEntry) -> Option<RuleEntry> {
        self.map.insert(key, entry)
    }

    /// Fast O(log n) value lookup. Returns the right-hand side expression only —
    /// use [`get_entry`][Self::get_entry] if you need the delayed flag too.
    pub fn get(&self, key: &Expr) -> Option<&Expr> {
        self.map.get(key).map(|e| &e.value)
    }

    /// Fast O(log n) entry lookup including the delayed flag.
    pub fn get_entry(&self, key: &Expr) -> Option<&RuleEntry> {
        self.map.get(key)
    }

    /// Whether `key` is present.
    pub fn contains_key(&self, key: &Expr) -> bool {
        self.map.contains_key(key)
    }

    /// Remove the entry for `key` and return it, if any.
    pub fn remove(&mut self, key: &Expr) -> Option<RuleEntry> {
        self.map.remove(key)
    }

    /// Iterate over entries in key order (BTreeMap iteration is sorted).
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, Expr, RuleEntry> {
        self.map.iter()
    }

    /// Borrow the underlying [`BTreeMap`].
    pub fn as_btree(&self) -> &BTreeMap<Expr, RuleEntry> {
        &self.map
    }

    /// Consume into the underlying [`BTreeMap`].
    pub fn into_btree(self) -> BTreeMap<Expr, RuleEntry> {
        self.map
    }
}

impl<'a> IntoIterator for &'a Association {
    type Item = (&'a Expr, &'a RuleEntry);
    type IntoIter = std::collections::btree_map::Iter<'a, Expr, RuleEntry>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl IntoIterator for Association {
    type Item = (Expr, RuleEntry);
    type IntoIter = std::collections::btree_map::IntoIter<Expr, RuleEntry>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl FromIterator<(Expr, RuleEntry)> for Association {
    fn from_iter<I: IntoIterator<Item = (Expr, RuleEntry)>>(iter: I) -> Self {
        Association {
            map: iter.into_iter().collect(),
        }
    }
}

impl FromIterator<(Expr, Expr)> for Association {
    /// Convenience: build an Association from `(key, value)` pairs treating each as
    /// an immediate `Rule` (not delayed).
    fn from_iter<I: IntoIterator<Item = (Expr, Expr)>>(iter: I) -> Self {
        Association {
            map: iter
                .into_iter()
                .map(|(k, v)| (k, RuleEntry::rule(v)))
                .collect(),
        }
    }
}

impl From<BTreeMap<Expr, RuleEntry>> for Association {
    fn from(map: BTreeMap<Expr, RuleEntry>) -> Self {
        Association { map }
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
    fn fast_get() {
        let mut a = Association::new();
        a.insert(Expr::from("k"), Expr::from(42));
        assert_eq!(a.get(&Expr::from("k")), Some(&Expr::from(42)));
        assert_eq!(a.get(&Expr::from("missing")), None);
    }

    #[test]
    fn rule_vs_rule_delayed() {
        let mut a = Association::new();
        a.insert(Expr::from("eager"), Expr::from(1));
        a.insert_delayed(Expr::from("lazy"), Expr::from(2));

        let eager = a.get_entry(&Expr::from("eager")).unwrap();
        assert!(!eager.delayed);
        assert_eq!(eager.value, Expr::from(1));

        let lazy = a.get_entry(&Expr::from("lazy")).unwrap();
        assert!(lazy.delayed);
        assert_eq!(lazy.value, Expr::from(2));
    }

    #[test]
    fn insert_overwrites_and_returns_old() {
        let mut a = Association::new();
        assert!(a.insert(Expr::from("k"), Expr::from(1)).is_none());
        let old = a.insert(Expr::from("k"), Expr::from(2)).unwrap();
        assert_eq!(old.value, Expr::from(1));
        assert_eq!(a.get(&Expr::from("k")), Some(&Expr::from(2)));
    }

    #[test]
    fn lookup_by_symbol() {
        let key = Expr::symbol(Symbol::new("Global`x"));
        let mut a = Association::new();
        a.insert(key.clone(), Expr::from(42));
        assert_eq!(a.get(&key), Some(&Expr::from(42)));
    }
}
