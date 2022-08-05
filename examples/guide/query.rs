use std::{
    borrow::BorrowMut,
    thread::sleep,
    time::{Duration, Instant},
};

use flax::{
    component, entities, CmpExt, CommandBuffer, Component, Debug, Entity, Mutable, Query,
    QueryData, Schedule, System, SystemContext, World, Write,
};
use itertools::Itertools;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;

fn main() -> color_eyre::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::default().with_indent_lines(true))
        .init();

    // ANCHOR: query_simple
    let mut world = World::new();

    component! {
        position: (f32, f32) => [ Debug ],
        health: f32 => [ Debug ],
    }

    // Spawn two entities
    let id = world.spawn();

    world.set(id, position(), (1.0, 4.0))?;
    world.set(id, health(), 100.0)?;

    let id2 = world.spawn();

    world.set(id2, position(), (-1.0, 4.0))?;
    world.set(id2, health(), 75.0)?;

    let mut query = Query::new((position(), health()));

    for (pos, health) in &mut query.prepare(&world) {
        println!("pos: {pos:?}, health: {health}");
    }

    // ANCHOR_END: query_simple
    // ANCHOR: query_modified

    component! {
        /// Distance to origin
        distance: f32 => [ flax::Debug ],
    }

    tracing::info!("Spawning id3");
    let id3 = world.spawn();
    world.set(id3, position(), (5.0, 6.0))?;
    world.set(id3, health(), 5.0)?;

    for id in [id, id2, id3] {
        tracing::info!("Adding distance to {id}");
        world.set(id, distance(), 0.0)?;
    }

    let mut query = Query::new((entities(), position(), distance().as_mut()))
        .filter(position().modified() & health().gt(0.0));

    tracing::info!("Updating distances");
    for (id, pos, dist) in &mut query.prepare(&world) {
        tracing::info!("Updating distance for {id} with position: {pos:?}");
        *dist = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
    }

    // ANCHOR_END: query_modified

    // ANCHOR: query_repeat

    tracing::info!("Running query again");
    for (id, pos, dist) in &mut query.prepare(&world) {
        tracing::info!("Updating distance for {id} with position: {pos:?}");
        *dist = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
    }
    // ANCHOR_END: query_repeat

    // ANCHOR: query_repeat_reboot

    *world.get_mut(id2, position())? = (8.0, 3.0);

    tracing::info!("... and again");
    for (id, pos, dist) in &mut query.prepare(&world) {
        tracing::info!("Updating distance for {id} with position: {pos:?}");
        *dist = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
    }

    // ANCHOR_END: query_repeat_reboot

    // ANCHOR: system_basic

    let mut update_dist = System::builder()
        .with_name("update distance")
        .with(query)
        .build(
            |mut query: QueryData<(_, Component<(f32, f32)>, Mutable<f32>), _>| {
                for (id, pos, dist) in &mut query.prepare() {
                    tracing::info!("Updating distance for {id} with position: {pos:?}");
                    *dist = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
                }
            },
        );

    update_dist.run_on(&mut world);
    // ANCHOR_END: system_basic

    // ANCHOR: system_for_each
    let mut update_dist = System::builder()
        .with_name("update distance")
        .with(
            Query::new((entities(), position(), distance().as_mut())).filter(position().modified()),
        )
        .for_each(|(id, pos, dist)| {
            tracing::info!("Updating distance for {id} with position: {pos:?}");
            *dist = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
        });

    for _ in 0..16 {
        update_dist.run_on(&mut world);
    }

    // ANCHOR_END: system_for_each

    // ANCHOR: system_cmd
    // Despawn all entities with a distance > 20
    // ANCHOR: schedule_basic
    let despawn = System::builder()
        .with_name("delete outside world")
        .with(Query::new((entities(), distance())).filter(distance().gt(20.0)))
        .with_cmd()
        .build(
            |mut query: QueryData<_, _>, mut cmd: Write<CommandBuffer>| {
                for (id, &dist) in &mut query.prepare() {
                    tracing::info!("Despawning {id} at: {dist}");
                    cmd.despawn(id);
                }
            },
        );

    let debug_world = System::builder()
        .with_name("debug world")
        .with_world()
        .build(|world: Write<World>| {
            tracing::debug!("World: {world:#?}");
        });

    // ANCHOR_END: system_cmd

    component! {
        is_static: () => [ flax::Debug ],
    }

    // Spawn 15 static entities, which wont move

    let mut rng = StdRng::seed_from_u64(42);

    for _ in 0..15 {
        let pos = (rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0));
        Entity::builder()
            .set(position(), pos)
            .set_default(distance())
            .set_default(is_static())
            .spawn(&mut world);
    }

    // Since this system will move non static entities out from the origin, they will
    // eventually be despawned
    let move_out = System::builder()
        .with_name("move_out")
        .with(Query::new(position().as_mut()).filter(is_static().without()))
        .for_each(|pos| {
            let mag = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();

            let dir = (pos.0 / mag, pos.1 / mag);

            pos.0 += dir.0;
            pos.1 += dir.1;
        });

    // let mut last_spawn = Instant::now();
    // let spawn_interval = Duration::from_secs(2);

    let spawn = System::builder().with_name("spawner").with_cmd().build(
        move |mut cmd: Write<CommandBuffer>| {
            let pos = (rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0));
            tracing::info!("Spawning new entity at: {pos:?}");
            Entity::builder()
                .set(position(), pos)
                .set_default(distance())
                .spawn_into(&mut cmd);
        },
    );

    let mut frame_count = 0;

    let count = System::builder()
        .with_name("count")
        .with(Query::new(()))
        .build(move |mut query: QueryData<()>| {
            let count: usize = query.prepare().iter_batched().map(|v| v.len()).sum();
            tracing::info!("[{frame_count}]: {count}");
            frame_count += 1;
        });

    let mut schedule = Schedule::builder()
        .with_system(update_dist)
        .with_system(despawn)
        .with_system(spawn)
        .with_system(move_out)
        .with_system(debug_world)
        .with_system(count)
        .build();

    tracing::info!("{schedule:#?}");

    for i in 0..200 {
        tracing::info!("Frame: {i}");
        schedule.execute_par(&mut world)?;
        sleep(Duration::from_secs_f32(0.1));
    }

    // ANCHOR_END: schedule_basic

    Ok(())
}