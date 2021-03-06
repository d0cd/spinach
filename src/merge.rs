use std::hash::Hash;
use std::collections::{ BTreeMap, BTreeSet, HashMap, HashSet };
use std::collections::hash_map;
use std::collections::btree_map;
use std::iter::Extend;

use std::cmp::Ordering;

/// Merge trait.
pub trait Merge {
    type Domain;

    // A "static" method.
    fn merge_in(val: &mut Self::Domain, other: Self::Domain);

    fn partial_cmp(val: &Self::Domain, other: &Self::Domain) -> Option<Ordering>;
}



// ORD MERGES //

pub struct Max<T: Ord> {
    _phantom: std::marker::PhantomData<T>,
}
impl <T: Ord> Merge for Max<T> {
    type Domain = T;

    fn merge_in(val: &mut T, other: T) {
        if *val < other {
            *val = other;
        }
    }

    fn partial_cmp(val: &T, other: &T) -> Option<Ordering> {
        val.partial_cmp(other)
    }
}

pub struct Min<T: Ord> {
    _phantom: std::marker::PhantomData<T>,
}
impl <T: Ord> Merge for Min<T> {
    type Domain = T;

    fn merge_in(val: &mut T, other: T) {
        if *val > other {
            *val = other;
        }
    }

    fn partial_cmp(val: &T, other: &T) -> Option<Ordering> {
        val.partial_cmp(other).map(|ord| ord.reverse())
    }
}

// SET MERGES //

pub struct Union<T> {
    _phantom: std::marker::PhantomData<T>,
}
impl <T: Eq + Hash> Merge for Union<HashSet<T>> {
    type Domain = HashSet<T>;

    fn merge_in(val: &mut HashSet<T>, other: HashSet<T>) {
        val.extend(other);
    }

    fn partial_cmp(val: &HashSet<T>, other: &HashSet<T>) -> Option<Ordering> {
        let s = val.union(other).count();
        if s != val.len() && s != other.len() {
            None
        }
        else if s == val.len() {
            if s == other.len() {
                Some(Ordering::Equal)
            }
            else {
                Some(Ordering::Greater)
            }
        }
        else {
            Some(Ordering::Less)
        }
    }
}
impl <T: Eq + Ord> Merge for Union<BTreeSet<T>> {
    type Domain = BTreeSet<T>;

    fn merge_in(val: &mut BTreeSet<T>, other: BTreeSet<T>) {
        val.extend(other);
    }

    fn partial_cmp(val: &BTreeSet<T>, other: &BTreeSet<T>) -> Option<Ordering> {
        let s = val.union(other).count();
        if s != val.len() && s != other.len() {
            None
        }
        else if s == val.len() {
            if s == other.len() {
                Some(Ordering::Equal)
            }
            else {
                Some(Ordering::Greater)
            }
        }
        else {
            Some(Ordering::Less)
        }
    }
}

pub struct Intersect<T> {
    _phantom: std::marker::PhantomData<T>,
}
impl <T: Eq + Hash> Merge for Intersect<HashSet<T>> {
    type Domain = HashSet<T>;

    fn merge_in(val: &mut HashSet<T>, other: HashSet<T>) {
        val.retain(|x| other.contains(x));
    }

    fn partial_cmp(val: &HashSet<T>, other: &HashSet<T>) -> Option<Ordering> {
        let s = val.intersection(other).count();
        if s != val.len() && s != other.len() {
            None
        }
        else if s == val.len() {
            if s == other.len() {
                Some(Ordering::Equal)
            }
            else {
                Some(Ordering::Greater)
            }
        }
        else {
            Some(Ordering::Less)
        }
    }
}
impl <T: Eq + Ord> Merge for Intersect<BTreeSet<T>> {
    type Domain = BTreeSet<T>;

    fn merge_in(val: &mut BTreeSet<T>, other: BTreeSet<T>) {
        // Not so ergonomic nor efficient.
        *val = other.into_iter()
            .filter(|x| val.contains(x))
            .collect();
    }

    fn partial_cmp(val: &BTreeSet<T>, other: &BTreeSet<T>) -> Option<Ordering> {
        let s = val.intersection(other).count();
        if s != val.len() && s != other.len() {
            None
        }
        else if s == val.len() {
            if s == other.len() {
                Some(Ordering::Equal)
            }
            else {
                Some(Ordering::Greater)
            }
        }
        else {
            Some(Ordering::Less)
        }
    }
}

// MAP MERGES //

pub struct MapUnion<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl <K, F> Merge for MapUnion<HashMap<K, F>>
where
    K: Hash + Eq,
    F: Merge,
{
    type Domain = HashMap<K, <F as Merge>::Domain>;

    fn merge_in(val: &mut Self::Domain, other: Self::Domain) {
        for (k, v) in other {
            match val.entry(k) {
                hash_map::Entry::Occupied(mut kv) => {
                    F::merge_in(kv.get_mut(), v);
                },
                hash_map::Entry::Vacant(kv) => {
                    kv.insert(v);
                },
            }
        }
    }

    // TODO: these are awful looking, and also need testing. Could use helper method.
    fn partial_cmp(val: &Self::Domain, other: &Self::Domain) -> Option<Ordering> {
        // Ordering::Equal OR Ordering::Greater
        if val.len() >= other.len() {
            let mut result = None;
            for (k, other_val) in other {
                match val.get(k) {
                    Some(val_val) => {
                        let cmp = F::partial_cmp(val_val, other_val);
                        match cmp {
                            Some(cmp) => {
                                if result.get_or_insert(cmp) != &cmp {
                                    return None;
                                }
                            },
                            None => return None,
                        }
                    },
                    None => return None,
                }
            }
            if None == result {
                return Some(Ordering::Equal);
            }
            else {
                return Some(Ordering::Greater);
            }
        }
        // Ordering::Less
        else {
            for (k, val_val) in val {
                match other.get(k) {
                    Some(other_val) => {
                        let cmp = F::partial_cmp(val_val, other_val);
                        if Some(Ordering::Less) != cmp {
                            return None;
                        }
                    },
                    None => return None,
                }
            }
            return Some(Ordering::Less);
        }
    }
}

impl <K, F> Merge for MapUnion<BTreeMap<K, F>>
where
    K: Ord + Eq,
    F: Merge,
{
    type Domain = BTreeMap<K, <F as Merge>::Domain>;

    fn merge_in(val: &mut Self::Domain, other: Self::Domain) {
        for (k, v) in other {
            match val.entry(k) {
                btree_map::Entry::Occupied(mut kv) => {
                    F::merge_in(kv.get_mut(), v);
                },
                btree_map::Entry::Vacant(kv) => {
                    kv.insert(v);
                },
            }
        }
    }

    // TODO: these are awful looking, and also need testing. Could use helper method.
    fn partial_cmp(val: &Self::Domain, other: &Self::Domain) -> Option<Ordering> {
        // Ordering::Equal OR Ordering::Greater
        if val.len() >= other.len() {
            let mut result = None;
            for (k, other_val) in other {
                match val.get(k) {
                    Some(val_val) => {
                        let cmp = F::partial_cmp(val_val, other_val);
                        match cmp {
                            Some(cmp) => {
                                if result.get_or_insert(cmp) != &cmp {
                                    return None;
                                }
                            },
                            None => return None,
                        }
                    },
                    None => return None,
                }
            }
            if None == result {
                return Some(Ordering::Equal);
            }
            else {
                return Some(Ordering::Greater);
            }
        }
        // Ordering::Less
        else {
            for (k, val_val) in val {
                match other.get(k) {
                    Some(other_val) => {
                        let cmp = F::partial_cmp(val_val, other_val);
                        if Some(Ordering::Less) != cmp {
                            return None;
                        }
                    },
                    None => return None,
                }
            }
            return Some(Ordering::Less);
        }
    }
}

// pub struct MapIntersection<T> {
//     _phantom: std::marker::PhantomData<T>,
// }
// impl <K, F> Merge for MapIntersection<HashMap<K, F>>
// where
//     K: Eq + Hash,
//     F: Merge,
// {
//     type Domain = HashMap<K, <F as Merge>::Domain>;

//     fn merge_in(val: &mut Self::Domain, other: Self::Domain) {
//         todo!("this is broken.");
//         for (k, v) in other {
//             val.entry(k).and_modify(|v0| F::merge_in(v0, v));
//         }
//     }

//     fn partial_cmp(val: &Self::Domain, other: &Self::Domain) -> Option<Ordering> {
//         todo!("this is broken.");
//         // Ordering::Equal OR Ordering::Less
//         if val.len() >= other.len() {
//             let mut result = None;
//             for (k, other_val) in other {
//                 match val.get(k) {
//                     Some(val_val) => {
//                         let cmp = F::partial_cmp(&val_val, other_val);
//                         match cmp {
//                             Some(cmp) => {
//                                 if result.get_or_insert(cmp) != &cmp {
//                                     return None;
//                                 }
//                             },
//                             None => return None,
//                         }
//                     },
//                     None => return None,
//                 }
//             }
//             if None == result {
//                 return Some(Ordering::Equal);
//             }
//             else {
//                 return Some(Ordering::Less);
//             }
//         }
//         // Ordering::Greater
//         else {
//             for (k, val_val) in val {
//                 match other.get(k) {
//                     Some(other_val) => {
//                         let cmp = F::partial_cmp(&val_val, other_val);
//                         if Some(Ordering::Greater) != cmp {
//                             return None;
//                         }
//                     },
//                     None => return None,
//                 }
//             }
//             return Some(Ordering::Greater);
//         }
//     }
// }

pub struct DominatingPair<AF, BF>
where
    AF: Merge,
    BF: Merge,
{
    _phantom: std::marker::PhantomData<(AF, BF)>,
}

impl <AF, BF> Merge for DominatingPair<AF, BF>
where
    AF: Merge,
    BF: Merge,
{
    type Domain = (<AF as Merge>::Domain, <BF as Merge>::Domain);

    fn merge_in(val: &mut Self::Domain, other: Self::Domain) {
        let cmp = AF::partial_cmp(&val.0, &other.0);
        match cmp {
            None => {
                AF::merge_in(&mut val.0, other.0);
                BF::merge_in(&mut val.1, other.1);
            },
            Some(Ordering::Equal) => {
                BF::merge_in(&mut val.1, other.1);
            },
            Some(Ordering::Less) => {
                *val = other;
            },
            Some(Ordering::Greater) => {},
        }
    }

    fn partial_cmp(val: &Self::Domain, other: &Self::Domain) -> Option<Ordering> {
        AF::partial_cmp(&val.0, &other.0).or_else(|| BF::partial_cmp(&val.1, &other.1))
    }
}







// Mingwei's weird semilattice.
// Merge is defined as, given signed integers A and B, take the value in the
// range [A, B] (or [B, A]) which is closest to zero.
// (Note that in general this will be A, B, or zero).
pub struct RangeToZeroI32;
impl Merge for RangeToZeroI32 {
    type Domain = i32;

    fn merge_in(val: &mut i32, other: i32) {
        if val.signum() != other.signum() {
            *val = 0;
        }
        else if val.abs() > other.abs() {
            *val = other
        }
    }

    fn partial_cmp(val: &i32, other: &i32) -> Option<Ordering> {
        if val.signum() != other.signum() {
            None
        }
        else {
            let less = val.abs().cmp(&other.abs());
            Some(less.reverse())
        }
    }
}
