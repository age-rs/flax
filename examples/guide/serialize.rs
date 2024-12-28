#[cfg(not(feature = "serde"))]
fn main() {}

#[cfg(feature = "serde")]
fn main() -> anyhow::Result<()> {
    use flax::{components::name, *};
    use glam::*;
    use rand::{distributions::Standard, rngs::StdRng, Rng, SeedableRng};
    use serde::de::DeserializeSeed;

    // ANCHOR: setup
    component! {
        position: Vec3 => [Debuggable],
        velocity: Vec3 => [Debuggable],
    }

    tracing_subscriber::fmt().init();

    use flax::serialize::{SerializationContextBuilder, SerializeFormat};
    tracing::info!("It works");

    let mut world = World::new();

    let mut rng = StdRng::seed_from_u64(239);

    let mut batch = BatchSpawn::new(16);

    batch.set(
        position(),
        (&mut rng).sample_iter(Standard).map(|v: Vec3| v * 2.0),
    )?;
    batch.set(velocity(), (&mut rng).sample_iter(Standard))?;
    batch.set(name(), (0..).map(|v| format!("id.{v}")))?;

    batch.spawn(&mut world);

    let mut batch = BatchSpawn::new(8);

    batch.set(
        position(),
        (&mut rng).sample_iter(Standard).map(|v: Vec3| v * 2.0),
    )?;
    batch.set(name(), (16..).map(|v| format!("id.{v}")))?;
    batch.spawn(&mut world);

    // ANCHOR_END: setup

    // ANCHOR: serialize
    let context = SerializationContextBuilder::new()
        .with(name())
        .with(position())
        .with(velocity())
        .build();

    let json =
        serde_json::to_string_pretty(&context.serialize_world(&world, SerializeFormat::RowMajor))?;

    // ANCHOR_END: serialize

    // ANCHOR: deserialize

    // An existing world with entities in it
    let mut world = World::new();

    let mut batch = BatchSpawn::new(32);

    batch.set(
        position(),
        (&mut rng).sample_iter(Standard).map(|v: Vec3| v * 2.0),
    )?;
    batch.set(name(), (0..).map(|v| format!("other_id.{v}")))?;
    batch.spawn(&mut world);

    let mut deserializer = serde_json::Deserializer::from_str(&json);
    let mut result = context.deserialize_world().deserialize(&mut deserializer)?;

    // Merge `result` into `world`
    world.merge_with(&mut result);

    // ANCHOR_END: deserialize

    Ok(())
}
