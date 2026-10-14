//! Камера преследования: инерция, FOV от скорости, тряска, взгляд назад, обход стен, камера смерти.

use crate::config::Settings;
use crate::effects::CameraShake;
use crate::game::*;
use crate::maps::MapData;
use crate::race::UiToggles;
use crate::vehicle::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

#[derive(Component)]
pub struct ChaseCam {
    pub pos: Vec3,
    pub look: Vec3,
    pub heading: Vec3,
    pub fov: f32,
    pub orbit: f32,
    pub init: bool,
    pub roll: f32,
}

impl Default for ChaseCam {
    fn default() -> Self {
        Self { pos: Vec3::new(0.0, 10.0, 20.0), look: Vec3::ZERO, heading: Vec3::NEG_Z, fov: 62.0, orbit: 0.0, init: false, roll: 0.0 }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Race), spawn_camera.after(crate::race::setup_race).run_if(not_headless))
            .add_systems(PostUpdate, chase_camera.before(TransformSystems::Propagate).run_if(in_state(AppState::Race)));
    }
}

pub fn spawn_camera(mut commands: Commands, map: Option<Res<MapData>>, settings: Res<Settings>) {
    let theme = map.as_ref().map(|m| m.theme).unwrap_or_else(|| crate::maps::builders::theme_for(crate::data::MapId::Port));
    let mut ec = commands.spawn((
        Camera3d::default(),
        Camera::default(),
        bevy::camera::Hdr,
        // слушатель для пространственного звука (крики пешеходов)
        SpatialListener::new(0.3),
        Projection::Perspective(PerspectiveProjection { fov: 64f32.to_radians(), near: 0.1, far: 3200.0, ..default() }),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ChaseCam::default(),
        DespawnOnExit(AppState::Race),
    ));
    crate::gfx::post_stack(&mut ec, &theme, settings.quality);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn chase_camera(
    time: Res<Time<Real>>,
    local: Res<LocalPlayer>,
    toggles: Res<UiToggles>,
    settings: Res<Settings>,
    mut shake: ResMut<CameraShake>,
    cars: Query<(&NetId, &Transform, &CarState, &Car), Without<ChaseCam>>,
    mut cam: Query<(&mut Transform, &mut ChaseCam, &mut Projection)>,
    spatial: SpatialQuery,
) {
    let Ok((mut tf, mut cc, mut proj)) = cam.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    // Цель: своя машина, а если её нет — первая живая
    let target = local
        .car
        .and_then(|id| cars.iter().find(|(n, ..)| **n == id))
        .or_else(|| cars.iter().find(|(_, _, st, _)| !st.wrecked))
        .or_else(|| cars.iter().next());
    let Some((_, ct, st, car)) = target else { return };
    let size = car.def().size;
    let car_pos = ct.translation;
    let fwd3 = ct.rotation * Vec3::NEG_Z;
    let fwd = horiz(fwd3).normalize_or(Vec3::NEG_Z);
    let speed = st.speed;

    let scale = (size.z / 4.4).clamp(1.0, 2.3) * settings.camera_distance;
    let (dist, height, look_h) = match toggles.camera_mode {
        1 => (10.5 * scale, 4.2 * scale, 1.5),
        2 => (0.0, size.y * 0.5 + 0.9, 0.0),
        _ => (7.4 * scale, 2.9 * scale, 1.3),
    };

    // Направление камеры догоняет машину; при заносе отстаёт сильнее
    let desired_heading = if speed < -3.0 { fwd } else { fwd };
    let follow = if st.air > 0.3 { 1.5 } else { 4.5 };
    if !cc.init {
        cc.heading = desired_heading;
    }
    cc.heading = cc.heading.lerp(desired_heading, 1.0 - (-follow * dt).exp()).normalize_or(desired_heading);
    let mut heading = cc.heading;
    if toggles.look_back {
        heading = -heading;
    }

    let mut desired_pos;
    let mut look;
    if st.wrecked {
        // Камера смерти: медленный облёт
        cc.orbit += dt * 0.45;
        let r = 12.0 * scale;
        desired_pos = car_pos + Vec3::new(cc.orbit.cos() * r, 5.0 * scale, cc.orbit.sin() * r);
        look = car_pos;
    } else if toggles.camera_mode == 2 {
        desired_pos = car_pos + ct.rotation * Vec3::new(0.0, height, -size.z * 0.15);
        look = desired_pos + (if toggles.look_back { -fwd3 } else { fwd3 }) * 20.0;
        tf.translation = desired_pos;
        cc.pos = desired_pos;
        cc.look = look;
        cc.init = true;
    } else {
        let speed_pull = (speed.abs() / 40.0).clamp(0.0, 1.0) * 1.2 * scale;
        desired_pos = car_pos - heading * (dist + speed_pull) + Vec3::Y * height;
        look = car_pos + Vec3::Y * look_h + heading * 3.0;
    }

    // Не прятаться за стенами
    if toggles.camera_mode != 2 {
        let from = car_pos + Vec3::Y * 1.2;
        let delta = desired_pos - from;
        let len = delta.length();
        if len > 0.5 {
            if let Ok(dir) = Dir3::new(delta / len) {
                let filter = SpatialQueryFilter::from_mask(Layer::World);
                if let Some(hit) = spatial.cast_ray(from, dir, len, true, &filter) {
                    let d = (hit.distance - 0.4).max(1.2);
                    desired_pos = from + *dir * d + Vec3::Y * 0.6;
                }
            }
        }
        if !cc.init {
            cc.pos = desired_pos;
            cc.look = look;
            cc.init = true;
        }
        let k_pos = 1.0 - (-9.0 * dt).exp();
        let k_look = 1.0 - (-14.0 * dt).exp();
        cc.pos = cc.pos.lerp(desired_pos, k_pos);
        // Не отставать слишком сильно на большой скорости
        let max_lag = dist * 1.8 + 4.0;
        if cc.pos.distance(desired_pos) > max_lag {
            cc.pos = desired_pos + (cc.pos - desired_pos).normalize() * max_lag;
        }
        cc.look = cc.look.lerp(look, k_look);
        look = cc.look;
    }

    // FOV растёт со скоростью и нитро
    let mut fov = 64.0 + (speed.abs() / 45.0).clamp(0.0, 1.0) * 14.0;
    if st.nitro_active {
        fov += 12.0;
    }
    cc.fov += (fov - cc.fov) * (1.0 - (-4.0 * dt).exp());
    if let Projection::Perspective(p) = proj.as_mut() {
        p.fov = cc.fov.to_radians();
    }

    // Тряска
    shake.0 = (shake.0 - dt * 2.2).max(0.0);
    let t = time.elapsed_secs();
    // удары + мелкая дрожь на большой скорости и с нитро
    let speed_buzz = ((speed.abs() - 22.0) / 30.0).clamp(0.0, 1.0) * 0.035 + if st.nitro_active { 0.03 } else { 0.0 };
    let s = shake.0 * shake.0 * 0.6 + speed_buzz;
    let jitter = Vec3::new((t * 47.0).sin() + (t * 23.0).cos() * 0.5, (t * 53.0).cos(), (t * 31.0).sin()) * s;
    if toggles.camera_mode != 2 {
        tf.translation = cc.pos + jitter;
    } else {
        tf.translation += jitter;
    }
    let target = look + jitter * 0.5;
    if (target - tf.translation).length_squared() > 1e-4 {
        tf.look_at(target, Vec3::Y);
    }
    // крен в повороте: камера наклоняется наружу
    let steer = (st.wheels[0].steer + st.wheels[1].steer) * 0.5;
    let roll_target = if st.wrecked || toggles.camera_mode == 2 { 0.0 } else { (steer * (speed.abs() / 30.0).min(1.2) * 0.09).clamp(-0.07, 0.07) };
    cc.roll += (roll_target - cc.roll) * (1.0 - (-5.0 * dt).exp());
    tf.rotate_local_z(cc.roll);
}
