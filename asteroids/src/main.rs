use ::rand::{rngs::StdRng, Rng, SeedableRng};
use anyhow::{Context, Result};
use flax::{
    events::{EventKind, EventSubscriber},
    filter::ChangeFilter,
    *,
};
use macroquad::{color::hsl_to_rgb, math::*, prelude::*};
use std::f32::consts::TAU;
use tracing_subscriber::{prelude::*, registry};
use tracing_tree::HierarchicalLayer;

// Declare the components that will be used.
component! {
    position: Vec2 => [ Debuggable ],
    rotation: f32 => [ Debuggable ],

    asteroid: () => [ Debuggable ],
    player: () => [ Debuggable ],

    /// Invincibility time in seconds
    invincibility: f32,

    /// The amount of material collected from asteroids
    material: f32 => [ Debuggable ],

    camera: Mat3 => [ Debuggable ],
    health: f32 => [ Debuggable ],
    color: Color => [ Debuggable ],
    mass: f32 => [ Debuggable ],
    difficulty: f32 => [Debuggable],

    velocity: Vec2=> [ Debuggable ],
    angular_velocity: f32 => [ Debuggable ],

    shape: Shape => [ Debuggable ],
    radius: f32 => [ Debuggable ],

    particle_size: f32,
    particle_lifetime: f32,

    on_collision: Box<dyn Fn(&World, Collision) + Send + Sync>,

    lifetime: f32 => [ Debuggable ],

    resources,
    rng: StdRng,

}

#[macroquad::main("Asteroids")]
async fn main() -> Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_span_retrace(true)
                .with_deferred_spans(true),
        )
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    let mut world = World::new();

    let rng = StdRng::seed_from_u64(42);
    world.set(resources(), self::rng(), rng).unwrap();

    let dt = 0.02;

    // Use a channel to subscribe to any player being despawned asynchronously
    let (player_dead_tx, player_dead_rx) = flume::unbounded();
    world.subscribe(
        player_dead_tx
            .filter_components([player().key()])
            .filter(|v, _| v == EventKind::Removed),
    );

    // Setup everything required for the game logic and physics
    //
    // Two different schedules will run independently of each other at different rates.
    let mut physics_schedule = Schedule::builder()
        .with_system(player_system(dt))
        .with_system(camera_system(dt))
        .with_system(lifetime_system(dt))
        .with_system(spawn_asteroids(64))
        .with_system(particle_system())
        .with_system(collision_system())
        .with_system(integrate_velocity(dt))
        .with_system(integrate_ang_velocity(dt))
        .with_system(despawn_out_of_bounds())
        .with_system(despawn_dead())
        .build();

    let mut frame_schedule = Schedule::builder()
        .with_system(draw_shapes())
        .with_system(draw_ui())
        .build();

    let mut acc = 0.0;

    create_player().spawn(&mut world);
    create_camera().spawn(&mut world);

    loop {
        if player_dead_rx.try_recv().is_ok() {
            world.despawn_many(asteroid().with());

            create_player().spawn(&mut world);
        }

        acc += get_frame_time();

        while acc > 0.0 {
            acc -= dt;
            let batches = physics_schedule.batch_info(&world);
            let batches = batches.to_names();
            tracing::debug!(?batches, "physics batches",);
            physics_schedule.execute_seq(&mut world)?;
        }

        match world.prune_archetypes() {
            0 => {}
            n => tracing::info!("Pruned {} archetypes", n),
        }

        clear_background(BLACK);

        frame_schedule.execute_seq(&mut world)?;

        next_frame().await
    }
}

const ASTEROID_SIZE: f32 = 40.0;

/// Create the central player ship
fn create_player() -> EntityBuilder {
    Entity::builder()
        .set_default(position())
        .set_default(rotation())
        .set_default(velocity())
        .set_default(angular_velocity())
        .set_default(invincibility())
        .set_default(player())
        .set(mass(), 100.0)
        .set(health(), 100.0)
        .set(
            shape(),
            Shape::Triangle(vec2(-8.0, 16.0), vec2(8.0, 16.0), vec2(0.0, -16.0)),
        )
        .set(radius(), 16.0)
        .set_default(material())
        .set(color(), GREEN)
        .set(
            on_collision(),
            Box::new(|world, collision| {
                let mut h = world.get_mut(collision.a, health()).unwrap();

                let mut invincibility = world.get_mut(collision.a, invincibility()).unwrap();

                if collision.impact > 10.0 && *invincibility <= 0.0 {
                    *invincibility = 1.0;
                    *h -= 20.0;
                }
            }),
        )
        .set(difficulty(), 1.0)
        .into()
}

fn create_camera() -> EntityBuilder {
    Entity::builder()
        .set_default(position())
        .set_default(velocity())
        .set_default(rotation())
        .set(camera(), Mat3::IDENTITY)
        .into()
}

const BULLET_DAMAGE: f32 = 20.0;
const BULLET_SPEED: f32 = 200.0;

fn create_bullet(player: Entity) -> EntityBuilder {
    Entity::builder()
        .set_default(velocity())
        .set_default(position())
        .set_default(rotation())
        .set(mass(), 10.0)
        .set(health(), 100.0)
        .set(shape(), Shape::Circle { radius: 4.0 })
        .set(radius(), 4.0)
        .set(lifetime(), 5.0)
        .set(color(), BLUE)
        .set(
            on_collision(),
            Box::new(move |world, coll| {
                *world.get_mut(coll.a, health()).unwrap() = 0.0;

                let Ok(mut health) = world.get_mut(coll.b, health()) else {
                    return;
                };

                if *health <= 0.0 {
                    return;
                }

                *health -= BULLET_DAMAGE;

                if *health <= 0.0 && player != coll.b {
                    if let Ok(mut material) = world.get_mut(player, material()) {
                        *material += world
                            .get_mut(coll.b, self::material())
                            .as_deref()
                            .copied()
                            .unwrap_or_default()
                    }
                }
            }),
        )
        .into()
}

fn create_particle(size: f32, lifetime: f32, color: Color) -> EntityBuilder {
    Entity::builder()
        .set_default(rotation())
        .set_default(position())
        .set(shape(), Shape::Circle { radius: size })
        .set(particle_size(), size)
        .set(self::lifetime(), lifetime)
        .set(particle_lifetime(), lifetime)
        .set(self::color(), color)
        .into()
}

fn create_explosion(
    rng: &mut StdRng,
    count: usize,
    pos: Vec2,
    speed: f32,
    size: f32,
    lifetime: f32,
    color: Color,
) -> impl Iterator<Item = EntityBuilder> + '_ {
    (0..count).map(move |_| {
        let dir = rng.gen_range(0.0..TAU);
        let speed = rng.gen_range(speed * 0.5..speed);

        create_particle(size, lifetime, color)
            .set(velocity(), speed * vec2(dir.cos(), dir.sin()))
            .set(position(), pos)
            .into()
    })
}

/// Updates each particle in the world
fn particle_system() -> BoxedSystem {
    System::builder()
        .with_name("particle_system")
        .with_query(Query::new((
            lifetime(),
            particle_size(),
            particle_lifetime(),
            shape().as_mut(),
        )))
        .for_each(|(lifetime, size, max_lifetime, shape)| {
            *shape = Shape::Circle {
                radius: lifetime / max_lifetime * size,
            };
        })
        .boxed()
}

#[derive(Fetch)]
struct CameraQuery {
    pos: ComponentMut<Vec2>,
    vel: ComponentMut<Vec2>,
    view: ComponentMut<Mat3>,
}

/// System which makes the camera track the player smoothly.
///
/// Uses two different queries, one for the player and one for the camera.
fn camera_system(dt: f32) -> BoxedSystem {
    System::builder()
        .with_query(Query::new((position(), velocity())).with(player()))
        .with_query(Query::new(CameraQuery {
            pos: position().as_mut(),
            vel: velocity().as_mut(),
            view: camera().as_mut(),
        }))
        .build(
            move |mut players: QueryBorrow<(Component<Vec2>, Component<Vec2>), _>,
                  mut cameras: QueryBorrow<CameraQuery>|
                  -> Result<()> {
                if let Some((player_pos, player_vel)) = players.first() {
                    for camera in &mut cameras {
                        *camera.pos = camera.pos.lerp(*player_pos, dt).lerp(*player_pos, dt);
                        *camera.vel = camera.vel.lerp(*player_vel, dt * 0.1);

                        let screen_size = vec2(screen_width(), screen_height());

                        *camera.view = Mat3::from_scale_angle_translation(
                            Vec2::ONE,
                            0.0,
                            *camera.pos - screen_size * 0.5,
                        )
                        .inverse();
                    }
                }
                Ok(())
            },
        )
        .boxed()
}

/// Macroquad has unsound race conditions, as such, use a mock shared
/// context
#[derive(Hash, Debug, Clone)]
struct GraphicsContext;

#[derive(Debug, Clone)]
enum Shape {
    Polygon { radius: f32, sides: u8 },
    Circle { radius: f32 },
    Triangle(Vec2, Vec2, Vec2),
}

impl Shape {
    pub fn draw(&self, view: &Mat3, pos: Vec2, rot: f32, color: Color) {
        match *self {
            Shape::Circle { radius } => {
                let pos = view.transform_point2(pos);
                let radius = view.transform_vector2(Vec2::splat(radius)).x;
                draw_circle(pos.x, pos.y, radius, color)
            }
            Shape::Polygon { radius, sides } => {
                let pos = view.transform_point2(pos);
                let radius = view.transform_vector2(Vec2::splat(radius)).x;

                draw_poly(pos.x, pos.y, sides, radius, rot, color)
            }
            Shape::Triangle(v1, v2, v3) => {
                let transform = *view * Mat3::from_scale_angle_translation(Vec2::ONE, rot, pos);

                let v1 = transform.transform_point2(v1);
                let v2 = transform.transform_point2(v2);
                let v3 = transform.transform_point2(v3);

                draw_triangle(v1, v2, v3, color)
            }
        }
    }
}

/// Represents a collision between two entities
struct Collision {
    a: Entity,
    b: Entity,
    dir: Vec2,
    depth: f32,
    impact: f32,
    system_mass: f32,
    point: Vec2,
}

#[derive(Fetch, Debug, Clone)]
struct CollisionQuery {
    pos: Component<Vec2>,
    vel: Component<Vec2>,
    mass: OptOr<Component<f32>, f32>,
    radius: Component<f32>,
}

impl CollisionQuery {
    pub fn new() -> Self {
        Self {
            pos: position(),
            vel: velocity(),
            mass: mass().opt_or_default(),
            radius: radius(),
        }
    }
}

fn lifetime_system(dt: f32) -> BoxedSystem {
    System::builder()
        .with_name("lifetime_system")
        .with_query(Query::new((entity_ids(), lifetime().as_mut())))
        .with_cmd_mut()
        .build(
            move |mut q: QueryBorrow<(EntityIds, ComponentMut<f32>)>, cmd: &mut CommandBuffer| {
                for (id, lf) in &mut q {
                    if *lf <= 0.0 {
                        cmd.set(id, health(), 0.0);
                    }
                    *lf -= dt;
                }
            },
        )
        .boxed()
}

/// N-body collision system
fn collision_system() -> BoxedSystem {
    System::builder()
        .with_name("collision_system")
        .with_query(Query::new(rng().as_mut()).entity(resources()))
        .with_query(Query::new((entity_ids(), CollisionQuery::new())))
        .with_query(Query::new((entity_ids(), CollisionQuery::new())))
        .with_world()
        .with_cmd_mut()
        .build(
            |mut resources: EntityBorrow<_>,
             mut a: QueryBorrow<(EntityIds, CollisionQuery)>,
             mut b: QueryBorrow<(EntityIds, CollisionQuery)>,
             world: &World,
             cmd: &mut CommandBuffer|
             -> Result<()> {
                let mut collisions = Vec::new();

                for (id_a, a) in &mut a {
                    for (id_b, b) in &mut b {
                        if id_a == id_b {
                            continue;
                        }

                        let radii = a.radius + b.radius;

                        let dir = *a.pos - *b.pos;
                        let depth = radii - dir.length();
                        let dir = dir.normalize_or_zero();

                        let impact = (*b.vel - *a.vel).dot(dir);

                        if impact > 0.0 && depth > 0.0 {
                            let system_mass = a.mass + b.mass;

                            collisions.push(Collision {
                                a: id_a,
                                b: id_b,
                                point: *a.pos + (*a.radius) * -dir,
                                dir,
                                depth,
                                impact,
                                system_mass,
                            });
                        }
                    }
                }

                // ensure there are no borrows when callbacks are executed
                drop((a, b));

                for collision in collisions {
                    let entity = world.entity(collision.a).unwrap();

                    {
                        let mut pos = entity.get_mut(position()).unwrap();
                        let mut vel = entity.get_mut(velocity()).unwrap();
                        let mass = *entity.get(mass()).unwrap();

                        *vel +=
                            collision.dir * collision.impact * (1.0 - mass / collision.system_mass);
                        *pos +=
                            collision.dir * collision.depth * (1.0 - mass / collision.system_mass);
                    }

                    let rng = resources.get().map_err(anyhow::Error::msg)?;
                    create_explosion(rng, 8, collision.point, collision.impact, 4.0, 1.0, GRAY)
                        .for_each(|v| {
                            cmd.spawn(v);
                        });

                    if let Ok(on_collision) = entity.get(on_collision()) {
                        (on_collision)(world, collision)
                    };
                }

                Ok(())
            },
        )
        .boxed()
}

const SHIP_THRUST: f32 = 150.0;
const SHIP_TURN: f32 = 2.0;
const WEAPON_COOLDOWN: f32 = 0.2;
const PLUME_COOLDOWN: f32 = 0.02;

/// Sometimes a query can grow to a very large tuple. Using a struct helps with naming the fields
/// and refactoring.
#[derive(Fetch)]
// Ensures the fetch item is debuggable
#[fetch(item_derives = [Debug])]
struct PlayerQuery {
    id: EntityIds,
    player: Component<()>,
    pos: Component<Vec2>,
    rot: ComponentMut<f32>,
    vel: ComponentMut<Vec2>,
    difficulty: ComponentMut<f32>,
    invincibility: ComponentMut<f32>,
    material: Component<f32>,
}

impl PlayerQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            player: player(),
            pos: position(),
            rot: rotation().as_mut(),
            vel: velocity().as_mut(),
            difficulty: difficulty().as_mut(),
            invincibility: invincibility().as_mut(),
            material: material(),
        }
    }
}

impl Default for PlayerQuery {
    fn default() -> Self {
        Self::new()
    }
}

fn player_system(dt: f32) -> BoxedSystem {
    let mut current_weapon_cooldown = 0.0;
    let mut current_plume_cooldown = 0.0;

    System::builder()
        .with_name("player_system")
        .with_query(Query::new(PlayerQuery::new()))
        .with_cmd_mut()
        .build(
            move |mut q: QueryBorrow<PlayerQuery>, cmd: &mut CommandBuffer| {
                current_weapon_cooldown -= dt;
                current_plume_cooldown -= dt;

                for player in &mut q {
                    *player.invincibility = (*player.invincibility - 0.02).max(0.0);

                    *player.difficulty = (*player.material * 0.001).max(1.0);

                    let forward = vec2(player.rot.sin(), -player.rot.cos());

                    let acc = if is_key_down(KeyCode::W) {
                        forward * SHIP_THRUST
                    } else if is_key_down(KeyCode::S) {
                        -forward * SHIP_THRUST
                    } else {
                        Vec2::ZERO
                    };

                    *player.vel += acc * dt;

                    if acc.length() > 0.0 && current_plume_cooldown <= 0.0 {
                        current_plume_cooldown = PLUME_COOLDOWN;
                        create_particle(8.0, 0.5, ORANGE)
                            .set(
                                position(),
                                *player.pos + *player.vel * dt - 30.0 * forward.normalize(),
                            )
                            .set(velocity(), *player.vel + -acc)
                            .spawn_into(cmd)
                    }

                    if is_key_down(KeyCode::A) {
                        *player.rot -= SHIP_TURN * dt;
                    }
                    if is_key_down(KeyCode::D) {
                        *player.rot += SHIP_TURN * dt;
                    }

                    if is_key_down(KeyCode::Space) && current_weapon_cooldown <= 0.0 {
                        current_weapon_cooldown = WEAPON_COOLDOWN;
                        create_bullet(player.id)
                            .set(velocity(), *player.vel + BULLET_SPEED * forward)
                            .set(position(), *player.pos + 30.0 * forward)
                            .spawn_into(cmd)
                    }
                }
            },
        )
        .boxed()
}

/// Kill of out of bounds entities relative to the player
fn despawn_out_of_bounds() -> BoxedSystem {
    System::builder()
        .with_name("despawn_out_of_bounds")
        .with_query(Query::new(position()).with(player()))
        .with_query(Query::new((position().modified(), health().as_mut())).without(player()))
        .build(
            |mut player: QueryBorrow<Component<Vec2>, _>,
             mut asteroids: QueryBorrow<(ChangeFilter<Vec2>, ComponentMut<f32>), _>| {
                if let Some(player_pos) = player.first() {
                    for (asteroid, health) in &mut asteroids {
                        if player_pos.distance(*asteroid) > 2500.0 {
                            *health = 0.0;
                        }
                    }
                }
            },
        )
        .boxed()
}

/// Deferred despawn dead entities (including players)
fn despawn_dead() -> BoxedSystem {
    System::builder()
        .with_name("despawn_dead")
        .with_query(Query::new(self::rng().as_mut()).entity(resources()))
        .with_query(
            Query::new((entity_ids(), position(), velocity(), material().opt()))
                .with_filter(health().modified() & health().le(0.0)),
        )
        .with_cmd_mut()
        .build(
            |mut resources: EntityBorrow<ComponentMut<StdRng>>,
             mut q: QueryBorrow<(EntityIds, Component<Vec2>, Component<_>, Opt<_>), _>,
             cmd: &mut CommandBuffer| {
                let rng = resources.get().unwrap();
                for (id, pos, vel, material) in &mut q {
                    cmd.despawn(id);
                    if let Some(mat) = material {
                        create_explosion(
                            &mut *rng,
                            (mat / 50.0) as _,
                            *pos,
                            50.0,
                            8.0,
                            4.0,
                            DARKPURPLE,
                        )
                        .for_each(|mut v| {
                            *v.get_mut(velocity()).unwrap() += *vel;
                            cmd.spawn(v);
                        });
                    }
                }
            },
        )
        .boxed()
}

/// Spawn random asteroids near the player up to a maximum concurrent count
fn spawn_asteroids(max_count: usize) -> BoxedSystem {
    System::builder()
        .with_name("spawn_asteroids")
        .with_query(Query::new(self::rng().as_mut()).entity(resources()))
        .with_query(Query::new((position(), difficulty())).with(player()))
        .with_query(Query::new(asteroid()))
        .with_cmd_mut()
        .build(
            move |mut resources: EntityBorrow<ComponentMut<StdRng>>,
                  mut players: QueryBorrow<(Component<Vec2>, Component<f32>), _>,
                  mut existing: QueryBorrow<Component<()>>,
                  cmd: &mut CommandBuffer| {
                let rng = resources.get().unwrap();

                let Some((player_pos, difficulty)) = players.first() else {
                    return;
                };

                let existing = existing.count();

                tracing::debug!(
                    ?existing,
                    max_count,
                    "Spawning asteroids around {player_pos}"
                );

                let mut builder = Entity::builder();

                (existing..max_count).for_each(|_| {
                    // Spawn around player
                    let dir = rng.gen_range(0f32..TAU);
                    let dir = vec2(dir.cos(), dir.sin());
                    let pos = *player_pos + dir * rng.gen_range(512.0..2048.0);

                    let size = rng.gen_range(0.2..1.0);
                    let radius = size * ASTEROID_SIZE;
                    let health = size * 100.0;

                    let dir = rng.gen_range(0f32..TAU);
                    let dir = vec2(dir.cos(), dir.sin());
                    let vel = dir * rng.gen_range(30.0..80.0) * difficulty.sqrt();

                    builder
                        .set(position(), pos)
                        .set(rotation(), rng.gen())
                        .set_default(asteroid())
                        .set(
                            shape(),
                            Shape::Polygon {
                                radius,
                                sides: rng.gen_range(3..16),
                            },
                        )
                        .set(mass(), radius * radius)
                        .set(self::material(), radius * radius)
                        .set(self::radius(), radius)
                        .set(self::health(), health)
                        .set(color(), hsl_to_rgb(0.75, 0.5, 0.5))
                        .set(velocity(), vel)
                        .set(angular_velocity(), rng.gen_range(-4.0..4.0))
                        .spawn_into(cmd);
                })
            },
        )
        .boxed()
}

fn integrate_velocity(dt: f32) -> BoxedSystem {
    System::builder()
        .with_name("integrate_velocity")
        .with_query(Query::new((position().as_mut(), velocity())))
        .for_each(move |(pos, vel)| {
            *pos += *vel * dt;
        })
        .boxed()
}

fn integrate_ang_velocity(dt: f32) -> BoxedSystem {
    System::builder()
        .with_name("integrate_ang_velocity")
        .with_query(Query::new((rotation().as_mut(), angular_velocity())))
        .for_each(move |(rot, w)| {
            *rot += *w * dt;
        })
        .boxed()
}

#[derive(Fetch, Debug, Clone)]
struct TransformQuery {
    pos: Component<Vec2>,
    rot: Component<f32>,
}

impl TransformQuery {
    fn new() -> Self {
        Self {
            pos: position(),
            rot: rotation(),
        }
    }
}

/// Draw each entity with a shape on the screen
fn draw_shapes() -> BoxedSystem {
    System::builder()
        .with_name("draw_asteroids")
        .with_resource(SharedResource::new(GraphicsContext))
        .with_query(Query::new(camera()))
        .with_query(Query::new((TransformQuery::new(), shape(), color())))
        .build(
            |_ctx: &mut GraphicsContext,
             mut camera: QueryBorrow<Component<Mat3>>,
             mut q: QueryBorrow<(TransformQuery, Component<Shape>, Component<Color>), _>|
             -> Result<()> {
                let view = camera.first().context("Missing camera")?;

                for (TransformQueryItem { pos, rot }, shape, color) in &mut q {
                    shape.draw(view, *pos, *rot, *color);
                }

                Ok(())
            },
        )
        .boxed()
}

/// Draws the score board by querying the ecs world for the data it needs.
///
/// For more complex Uis, consider having a look at [`violet`](https://github.com/ten3roberts/violet)
/// which uses `flax`
fn draw_ui() -> BoxedSystem {
    System::builder()
        .with_name("draw_ui")
        .with_resource(SharedResource::new(GraphicsContext))
        .with_query(Query::new((material(), health(), difficulty())).with(player()))
        .with_query(Query::new(()))
        .with_world()
        .build(
            |_ctx: &mut GraphicsContext,
             mut players: QueryBorrow<(Component<f32>, Component<f32>, Component<f32>), _>,
             mut all: QueryBorrow<(), _>,
             world: &World| {
                let count = all.count();

                let result = players.first();

                if let Some((material, health, _difficulty)) = result {
                    draw_text(
                        &format!("Hull: {}%", health.round()),
                        10.0,
                        32.0,
                        32.0,
                        Color::from_vec(
                            vec4(1.0, 0.0, 0.0, 1.0).lerp(vec4(0.0, 1.0, 0.0, 1.0), health / 100.0),
                        ),
                    );

                    draw_text(
                        &format!("Material: {}kg", material.round()),
                        10.0,
                        64.0,
                        32.0,
                        BLUE,
                    );

                    draw_text(&format!("Entities: {count}"), 10.0, 96.0, 16.0, GRAY);

                    draw_text(
                        &format!(
                            "Archetype Gen: {}, Change Tick: {}, Frametime: {}",
                            world.archetype_gen(),
                            world.change_tick(),
                            get_frame_time(),
                        ),
                        10.0,
                        128.0,
                        16.0,
                        GRAY,
                    );

                    // draw_rectangle(10.0, 10.0, 256.0, 16.0, DARKPURPLE);
                    // draw_rectangle(
                    //     10.0,
                    //     10.0,
                    //     256.0 * (player_health / 100.0).clamp(0.0, 1.0),
                    //     16.0,
                    //     GREEN,
                    // );
                }
            },
        )
        .boxed()
}
