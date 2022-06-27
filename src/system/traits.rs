use std::marker::PhantomData;

use atomic_refcell::{AtomicRef, AtomicRefMut};
use thiserror::private::PathAsDisplay;

use crate::World;

use super::{cell::SystemContext, Access};

/// Describe an access to the world in ters of shared and unique accesses
pub trait WorldAccess {
    /// Returns all the accesses for a system
    fn access(&mut self, world: &World) -> Vec<Access>;
}

/// Describes a type which can run on a set of system data.
///
/// Is implemented for functions up to an arity of 8
pub trait SystemFn<'w, Args, Ret>
where
    Ret: 'static,
{
    fn execute(&mut self, ctx: &'w SystemContext<'w>, data: &'w mut Args) -> Ret;
}

macro_rules! tuple_impl {
    ($($idx: tt => $ty: ident),*) => {
        // Fallible
        // impl<'w, Func, $($ty,)*T, Err> SystemFn<'w, ($($ty,)*), Result<T, Err>> for Func
        // where
        //     Func: Fn($(<$ty as SystemData<'w>>::Prepared,)*) -> Result<T, Err>,
        //     $($ty: SystemData<'w>,)*
        // {
        //     fn execute<'a>(&mut self, world: &'w World, data: &'w mut ($($ty,)*)) -> Result<T, Err> {
        //         let _prepared = data.prepare_data(world);
        //         (self)($((_prepared.$idx),)*).into()
        //     }
        // }

        // Infallible
        impl<'w, Func, $($ty,)* Ret> SystemFn<'w, ($($ty,)*), Ret> for Func
        where
            Func: for<'x, 'y> FnMut($(<$ty as SystemData<'x, 'y, 'w>>::Data,)*) -> Ret,
            Ret: 'static,
            $($ty: for<'x> SystemData<'x, 'w>,)*
        {
            fn execute<'a>(&mut self, world: &'w SystemContext, data: &'w mut ($($ty,)*)) -> Ret {
                let mut init = <($($ty,)*) as SystemData>::init(world);
                {
                    let data = data.bind(&mut init);
                    (self)($((data.$idx),)*);
                }

                todo!()
            }
        }

        impl<'init, 'w, $($ty,)*> SystemData<'init, 'w> for ($($ty,)*)
        where
            $($ty: SystemData<'init, 'w>,)*
        {
            type Init = ($(<$ty as SystemData<'init, 'w>>::Init,)*);
            type Data = ($(<$ty as SystemData<'init, 'w>>::Data,)*);
            fn init(_ctx: &'w SystemContext) -> Self::Init {
                ($(<$ty>::init(_ctx),)*)
            }

            fn bind(&mut self, init: &'init mut Self::Init) -> Self::Data {
                ($((self.$idx).bind(&mut init.$idx),)*)
            }
        }
    };
}

// tuple_impl! {}
// tuple_impl! { 0 => A }
// tuple_impl! { 0 => A, 1 => B }
// tuple_impl! { 0 => A, 1 => B, 2 => C }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => H }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => H, 7 => I }
// tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => H, 7 => I, 8 => J }

// pub trait SystemData<'init, 'ctx, 'w> {
//     type Init;
//     /// Initialize and fetch data from the system execution context
//     fn init(ctx: &'ctx SystemContext<'w>) -> Self::Init;
// }

/// Describes data needed for a system execution which can construct itself from
/// a guard returned by the system context.
pub trait SystemData<'a> {
    type Output;
    fn bind(&'a mut self) -> Self::Output;
}

/// Fetches the appropriate guards from a system context
pub trait SystemDataInit<'a> {
    type Output;
    fn init(&'a mut self, ctx: &'a SystemContext<'a>) -> Self::Output;
}

struct Write<T>(PhantomData<T>);

impl<T> Write<T> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, 'w> SystemData<'a> for AtomicRefMut<'w, &'w mut World> {
    type Output = &'a mut World;

    fn bind(&'a mut self) -> Self::Output {
        &mut ***self
    }
}

impl<'a> SystemDataInit<'a> for Write<World> {
    type Output = AtomicRefMut<'a, &'a mut World>;
    fn init(&'a mut self, ctx: &'a SystemContext<'a>) -> Self::Output {
        ctx.world_mut().unwrap()
    }
}

impl<'w, F> SystemFn<'w, Write<World>, ()> for F
where
    F: for<'x> FnMut(&'x mut World),
{
    fn execute(&mut self, ctx: &'w SystemContext<'w>, data: &'w mut Write<World>) {
        let mut init = data.init(ctx);
        let data = init.bind();
        (self)(data)
    }
}

#[cfg(test)]
mod test {
    use crate::{system::SystemContext, CommandBuffer, World};

    use super::{SystemFn, Write};

    #[test]
    fn system_fn() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let ctx = SystemContext::new(&mut world, &mut cmd);

        fn func(world: &mut World) {}
        // let mut func = Box::new(func);
        let mut func = &mut func;

        // (func).execute(&ctx, &mut (Write::<World>::new(),))
    }
}