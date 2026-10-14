//! Витрина в меню: машина игрока на поворотном круге.

use super::garage::GarageState;
use crate::carmodel::{build_car_model, ModelAssets};
use crate::config::*;
use crate::game::*;
use crate::vehicle::*;
use bevy::post_process::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;

#[derive(Component)]
pub struct ShowroomCar {
    pub shown: Option<CarConfig>,
}

#[derive(Component)]
struct Turntable;

pub struct ShowroomPlugin;

impl Plugin for ShowroomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), spawn_showroom)
            .add_systems(Update, (update_showroom_car, spin).run_if(in_state(AppState::Menu)));
    }
}

fn spawn_showroom(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(ClearColor(Color::srgb(0.035, 0.035, 0.045)));
    commands.insert_resource(GlobalAmbientLight { color: Color::srgb(0.8, 0.85, 1.0), brightness: 220.0, ..default() });
    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        bevy::camera::Hdr,
        Tonemapping::AgX,
        bevy::camera::Exposure { ev100: 9.2 },
        Bloom { intensity: 0.15, ..Bloom::NATURAL },
        Transform::from_xyz(-3.8, 2.6, 8.6).looking_at(Vec3::new(-2.6, 0.7, 0.0), Vec3::Y),
        bevy::pbr::DistanceFog { color: Color::srgb(0.035, 0.035, 0.045), falloff: bevy::pbr::FogFalloff::Linear { start: 14.0, end: 40.0 }, ..default() },
        DespawnOnExit(AppState::Menu),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 6000.0, shadow_maps_enabled: true, color: Color::srgb(1.0, 0.95, 0.88), ..default() },
        Transform::default().looking_to(Vec3::new(-0.5, -1.0, -0.6), Vec3::Y),
        DespawnOnExit(AppState::Menu),
    ));
    for (p, c) in [(Vec3::new(4.0, 5.0, 3.0), Color::srgb(1.0, 0.55, 0.15)), (Vec3::new(-5.0, 4.0, -3.0), Color::srgb(0.3, 0.5, 1.0))] {
        commands.spawn((
            PointLight { color: c, intensity: 600_000.0, range: 30.0, shadow_maps_enabled: false, ..default() },
            Transform::from_translation(p),
            DespawnOnExit(AppState::Menu),
        ));
    }
    // Пол и поворотный круг
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(600.0, 600.0))),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.07, 0.07, 0.08), perceptual_roughness: 0.35, metallic: 0.3, ..default() })),
        Transform::from_xyz(0.0, -0.02, 0.0),
        DespawnOnExit(AppState::Menu),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(4.6, 0.12).mesh().resolution(64))),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.16, 0.16, 0.18), perceptual_roughness: 0.25, metallic: 0.7, ..default() })),
        Transform::from_xyz(0.0, 0.04, 0.0),
        DespawnOnExit(AppState::Menu),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(4.55, 4.7).mesh())),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::BLACK, emissive: LinearRgba::rgb(6.0, 2.4, 0.3), ..default() })),
        Transform::from_xyz(0.0, 0.1, 0.0),
        DespawnOnExit(AppState::Menu),
    ));
    commands.spawn((Transform::default(), Visibility::default(), Turntable, ShowroomCar { shown: None }, DespawnOnExit(AppState::Menu)));
}

fn update_showroom_car(
    mut commands: Commands,
    mut q: Query<(Entity, &mut ShowroomCar, &mut Transform)>,
    profile: Res<Profile>,
    gs: Res<GarageState>,
    wear: Option<Res<crate::carmodel::WearAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((e, mut sc, mut tf)) = q.single_mut() else { return };
    let cfg = gs.shown_config(&profile);
    if sc.shown.as_ref() == Some(&cfg) {
        return;
    }
    let stats = CarStats::compute(&cfg);
    commands.entity(e).despawn_related::<Children>();
    let mut a = ModelAssets { meshes: &mut meshes, materials: &mut mats, wear: wear.as_deref().cloned(), headlights: false };
    commands.entity(e).with_children(|c| build_car_model(c, &mut a, &cfg));
    // Колёса в положение покоя
    tf.translation.y = stats.ride_height + 0.1;
    let scale = (4.6 / cfg.def().size.z.max(3.0)).min(1.0);
    tf.scale = Vec3::splat(scale.max(0.55));
    tf.translation.y *= tf.scale.y;
    commands.entity(e).insert(CarState { wheels: [WheelVis { comp: stats.rest_len * 0.42, grounded: true, ..default() }; 4], ..default() });
    commands.entity(e).insert(Car { name: String::new(), controller: Controller::Bot, cfg: cfg.clone(), bot_seed: 0 });
    sc.shown = Some(cfg);
}

fn spin(time: Res<Time>, mut q: Query<&mut Transform, With<Turntable>>) {
    for mut t in q.iter_mut() {
        t.rotate_y(time.delta_secs() * 0.35);
    }
}
