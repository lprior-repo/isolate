//! Functional programming patterns and utilities.
//!
//! This module demonstrates idiomatic functional Rust patterns
//! combined with zero-panic error handling.

use std::collections::HashMap;

use crate::Result;

/// A pure function that transforms input to output with potential errors.
pub type FallibleTransform<T, U> = fn(T) -> Result<U>;

/// A pure function that validates input.
pub type Validator<T> = fn(&T) -> Result<()>;

/// Applies a sequence of validators to an item.
///
/// All validators must pass for the operation to succeed.
///
/// # Examples
///
/// ```ignore
/// let validators: Vec<_> = vec![
///     &validate_name as &dyn Fn(&String) -> Result<()>,
///     &validate_length,
/// ];
///
/// validate_all("test_name", &validators)?;
/// ```
pub fn validate_all<T>(item: &T, validators: &[&dyn Fn(&T) -> Result<()>]) -> Result<()> {
    validators
        .iter()
        .try_fold((), |_, validator| validator(item))
}

/// Composes two fallible transformations into one.
pub fn compose_result<T, U, V>(
    f: impl Fn(T) -> Result<U>,
    g: impl Fn(U) -> Result<V>,
) -> impl Fn(T) -> Result<V> {
    move |x| f(x).and_then(|y| g(y))
}

/// Applies transformers sequentially, chaining results.
pub fn apply_transforms<T>(item: T, transforms: &[&dyn Fn(T) -> Result<T>]) -> Result<T> {
    transforms
        .iter()
        .try_fold(item, |acc, transform| transform(acc))
}

/// Groups items by a key function.
pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq,
    F: Fn(&T) -> K,
{
    items.into_iter().fold(HashMap::new(), |mut map, item| {
        let key = key_fn(&item);
        map.entry(key).or_insert_with(Vec::new).push(item);
        map
    })
}

/// Partitions items based on a predicate.
pub fn partition<T, F>(items: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
where
    F: Fn(&T) -> bool,
{
    items.into_iter().partition(predicate)
}

/// Folds a sequence with error handling.
pub fn fold_result<T, U, F>(items: Vec<T>, init: U, f: F) -> Result<U>
where
    F: Fn(U, T) -> Result<U>,
{
    items.into_iter().try_fold(init, f)
}

/// Maps and collects, short-circuiting on first error.
pub fn map_result<T, U, F>(items: Vec<T>, f: F) -> Result<Vec<U>>
where
    F: Fn(T) -> Result<U>,
{
    items.into_iter().map(f).collect()
}

/// Filters items with a fallible predicate.
pub fn filter_result<T, F>(items: Vec<T>, f: F) -> Result<Vec<T>>
where
    F: Fn(&T) -> Result<bool>,
{
    items.into_iter().try_fold(Vec::new(), |mut acc, item| {
        f(&item).map(|keep| {
            if keep {
                acc.push(item);
            }
            acc
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    fn is_positive(n: &i32) -> Result<()> {
        if *n > 0 {
            Ok(())
        } else {
            Err(Error::ValidationError("not positive".into()))
        }
    }

    fn is_even(n: &i32) -> Result<()> {
        if n % 2 == 0 {
            Ok(())
        } else {
            Err(Error::ValidationError("not even".into()))
        }
    }

    #[test]
    fn test_validate_all_success() {
        let validators: Vec<&dyn Fn(&i32) -> Result<()>> = vec![&is_positive, &is_even];
        assert!(validate_all(&4, &validators).is_ok());
    }

    #[test]
    fn test_validate_all_failure() {
        let validators: Vec<&dyn Fn(&i32) -> Result<()>> = vec![&is_positive, &is_even];
        assert!(validate_all(&3, &validators).is_err());
    }

    #[test]
    fn test_compose_result() {
        let double = |x: i32| -> Result<i32> { Ok(x * 2) };
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let composed = compose_result(double, add_one);

        assert_eq!(composed(5).unwrap_or_default(), 11); // (5 * 2) + 1
    }

    #[test]
    fn test_group_by() {
        let items = vec![("a", 1), ("b", 2), ("a", 3), ("b", 4)];
        let grouped = group_by(items, |(key, _)| key);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped["a"].len(), 2);
        assert_eq!(grouped["b"].len(), 2);
    }

    #[test]
    fn test_partition() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let (even, odd) = partition(items, |x| x % 2 == 0);

        assert_eq!(even, vec![2, 4, 6]);
        assert_eq!(odd, vec![1, 3, 5]);
    }

    #[test]
    fn test_fold_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = fold_result(items, 0, |acc, x| Ok(acc + x));
        assert_eq!(result.unwrap_or_default(), 15);
    }

    #[test]
    fn test_map_result() {
        let items = vec![1, 2, 3];
        let result = map_result(items, |x| Ok(x * 2));
        assert_eq!(result.unwrap_or_default(), vec![2, 4, 6]);
    }

    #[test]
    fn test_filter_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = filter_result(items, |x| Ok(x % 2 == 0));
        assert_eq!(result.unwrap_or_default(), vec![2, 4]);
    }
}
