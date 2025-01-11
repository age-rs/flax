use core::{
    fmt::{self, Formatter},
    ops::Deref,
};

use alloc::vec::Vec;

use crate::{
    archetype::{Slice, Slot},
    system::Access,
    Fetch, FetchItem,
};

use super::{FetchAccessData, FetchPrepareData, PreparedFetch, RandomFetch, TransformFetch};

#[derive(Debug, Clone)]
/// Component which cloned the value.
///
/// This is useful as the query item is 'static
/// See [crate::Component::as_mut]
pub struct Cloned<F>(pub F);

impl<'q, F> FetchItem<'q> for Cloned<F>
where
    F: FetchItem<'q>,
    <F as FetchItem<'q>>::Item: Deref,
    <<F as FetchItem<'q>>::Item as Deref>::Target: 'static + Clone,
{
    type Item = <<F as FetchItem<'q>>::Item as Deref>::Target;
}

impl<'w, F> Fetch<'w> for Cloned<F>
where
    F: Fetch<'w>,
    F: for<'q> FetchItem<'q>,
    for<'q> <F as FetchItem<'q>>::Item: Deref,
    for<'q> <<F as FetchItem<'q>>::Item as Deref>::Target: 'static + Clone,
{
    const MUTABLE: bool = F::MUTABLE;

    type Prepared = Cloned<F::Prepared>;

    #[inline(always)]
    fn prepare(&'w self, data: FetchPrepareData<'w>) -> Option<Self::Prepared> {
        Some(Cloned(self.0.prepare(data)?))
    }

    fn filter_arch(&self, data: FetchAccessData) -> bool {
        self.0.filter_arch(data)
    }

    fn access(&self, data: FetchAccessData, dst: &mut Vec<Access>) {
        self.0.access(data, dst)
    }

    fn describe(&self, f: &mut Formatter) -> fmt::Result {
        self.0.describe(f)
    }

    #[inline]
    fn searcher(&self, searcher: &mut crate::ArchetypeSearcher) {
        self.0.searcher(searcher)
    }
}

impl<'q, F, V> PreparedFetch<'q> for Cloned<F>
where
    F: PreparedFetch<'q>,
    F::Item: Deref<Target = V>,
    V: 'static + Clone,
{
    type Item = V;
    type Chunk = F::Chunk;

    const HAS_FILTER: bool = F::HAS_FILTER;

    unsafe fn create_chunk(&'q mut self, slots: Slice) -> Self::Chunk {
        self.0.create_chunk(slots)
    }

    unsafe fn fetch_next(chunk: &mut Self::Chunk) -> Self::Item {
        F::fetch_next(chunk).clone()
    }

    unsafe fn filter_slots(&mut self, slots: Slice) -> Slice {
        self.0.filter_slots(slots)
    }
}

impl<'q, V, F> RandomFetch<'q> for Cloned<F>
where
    F: RandomFetch<'q>,
    F::Item: Deref<Target = V>,
    V: 'static + Clone,
{
    unsafe fn fetch_shared(&'q self, slot: Slot) -> Self::Item {
        self.0.fetch_shared(slot).clone()
    }

    unsafe fn fetch_shared_chunk(chunk: &Self::Chunk, slot: Slot) -> Self::Item {
        F::fetch_shared_chunk(chunk, slot).clone()
    }
}

impl<K, F> TransformFetch<K> for Cloned<F>
where
    F: TransformFetch<K>,
    Cloned<F>: for<'x> Fetch<'x>,
    Cloned<F::Output>: for<'x> Fetch<'x>,
{
    type Output = Cloned<F::Output>;

    fn transform_fetch(self, method: K) -> Self::Output {
        Cloned(self.0.transform_fetch(method))
    }
}
