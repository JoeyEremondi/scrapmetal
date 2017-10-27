use super::{Cast, Term};
use std::marker::PhantomData;

/// A similar work around as `GenericTransform`, but mutating in place and
/// optionally returning some query type, rather than taking `self` and
/// returning the same `Self` type.
///
/// This is roughly equivalent to `for<T> FnMut(&mut T) -> R`.
pub trait GenericMutate<R> {
    /// Call the query function on any `T`.
    fn mutate<T>(&mut self, t: &mut T) -> R
    where
        T: Term;
}

/// A mutation creates some value `R` from mutable references to a `U`. It can
/// be called on values of any type `T`, not just on values of type `U`, so it
/// requires a default `R` value for when it is called on values which are not a
/// `T`.
///
/// This lifts an `FnMut(&mut U) -> R` into a `for<T> FnMut(&mut T) -> R`.
#[derive(Debug)]
pub struct Mutation<M, U, D, R>
where
    M: FnMut(&mut U) -> R,
    D: FnMut() -> R,
{
    make_default: D,
    mutation: M,
    phantom: PhantomData<fn(&mut U) -> R>,
}

impl<M, U, R> Mutation<M, U, fn() -> R, R>
where
    M: FnMut(&mut U) -> R,
    R: Default,
{
    /// Construct a new `Mutation`, returning `R::default()` for the cases where we
    /// query a value whose type is not `U`.
    #[inline]
    pub fn new(mutation: M) -> Mutation<M, U, fn() -> R, R> {
        Mutation {
            make_default: Default::default,
            mutation,
            phantom: PhantomData,
        }
    }
}

impl<M, U, D, R> Mutation<M, U, D, R>
where
    M: FnMut(&mut U) -> R,
    D: FnMut() -> R,
{
    /// Construct a new `Mutation`, returning `make_default()` for the cases where
    /// we query a value whose type is not `U`.
    #[inline]
    pub fn or_else(make_default: D, mutation: M) -> Mutation<M, U, D, R> {
        Mutation {
            make_default,
            mutation,
            phantom: PhantomData,
        }
    }
}

impl<M, U, D, R> GenericMutate<R> for Mutation<M, U, D, R>
where
    M: FnMut(&mut U) -> R,
    D: FnMut() -> R,
{
    #[inline]
    fn mutate<T>(&mut self, t: &mut T) -> R
    where
        T: Term,
    {
        match Cast::<&mut U>::cast(t) {
            Ok(u) => (self.mutation)(u),
            Err(_) => (self.make_default)(),
        }
    }
}

/// Recursively perform a query in a top-down, left-to-right manner across a
/// data structure. The `M: GenericMutate<R>` queries individual values, while the `F:
/// FnMut(R, R) -> R` joins the results of multiple queries into a single
/// result.
#[derive(Debug)]
pub struct MutateEverything<M, R, F>
where
    M: GenericMutate<R>,
    F: FnMut(R, R) -> R,
{
    m: M,
    fold: F,
    phantom: PhantomData<fn(R, R) -> R>,
}

impl<M, R, F> MutateEverything<M, R, F>
where
    M: GenericMutate<R>,
    F: FnMut(R, R) -> R,
{
    /// Construct a new `MutateEverything` query traversal.
    #[inline]
    pub fn with_query(m: M, fold: F) -> MutateEverything<M, R, F> {
        MutateEverything {
            m,
            fold,
            phantom: PhantomData,
        }
    }
}

impl<M> MutateEverything<M, (), fn((), ())>
where
    M: GenericMutate<()>,
{
    /// Construct a new `MutateEverything` query traversal.
    #[inline]
    pub fn new(m: M) -> MutateEverything<M, (), fn((), ())> {
        #[inline(always)]
        fn fold(_: (), _: ()) {}
        MutateEverything {
            m,
            fold,
            phantom: PhantomData,
        }
    }
}

impl<M, R, F> GenericMutate<R> for MutateEverything<M, R, F>
where
    M: GenericMutate<R>,
    F: FnMut(R, R) -> R,
{
    #[inline]
    fn mutate<T>(&mut self, t: &mut T) -> R
    where
        T: Term,
    {
        let mut r = Some(self.m.mutate(t));
        t.map_one_mutation(self, |me, rr| {
            r = Some((me.fold)(r.take().unwrap(), rr));
        });
        r.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating() {
        let mut set_char_to_a = Mutation::new(|c: &mut char| {
            *c = 'a';
            1
        });
        let mut char = 'b';
        assert_eq!(set_char_to_a.mutate(&mut char), 1);
        assert_eq!(char, 'a');

        let mut v = vec![1, 2, 3];
        assert_eq!(set_char_to_a.mutate(&mut v), 0);
    }
}
