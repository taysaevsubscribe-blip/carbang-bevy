//! Сгенерировано tools/cars/convert.py из Kenney Car Kit (CC0). Не править вручную.
//! Габариты моделей в метрах, координаты машины: перед -Z, высота от оси колёс.
use bevy::prelude::*;

pub struct CarFit {
    pub id: &'static str,
    /// центры колёс модели: перед-лево, перед-право, зад-лево, зад-право
    pub wheels: [Vec3; 4],
    /// радиус колёс модели: перед, зад
    pub wheel_r: [f32; 2],
    pub min: Vec3,
    pub max: Vec3,
    pub roof_y: f32,
}

pub const FITS: &[CarFit] = &[
    CarFit { id: "ferret", wheels: [Vec3::new(-0.582, 0.000, -1.144), Vec3::new(0.582, 0.000, -1.144), Vec3::new(-0.582, 0.000, 1.075), Vec3::new(0.582, 0.000, 1.075)], wheel_r: [0.411, 0.411], min: Vec3::new(-0.890, -0.206, -1.952), max: Vec3::new(0.890, 1.096, 1.952), roof_y: 1.096 },
    CarFit { id: "icecream", wheels: [Vec3::new(-0.630, 0.000, -1.511), Vec3::new(0.630, 0.000, -1.511), Vec3::new(-0.630, 0.000, 0.854), Vec3::new(0.630, 0.000, 0.854)], wheel_r: [0.438, 0.438], min: Vec3::new(-1.050, -0.219, -2.372), max: Vec3::new(1.050, 1.971, 2.372), roof_y: 1.898 },
    CarFit { id: "buggy", wheels: [Vec3::new(-0.719, 0.000, -0.402), Vec3::new(0.719, 0.000, -0.402), Vec3::new(-0.719, 0.000, 0.898), Vec3::new(0.719, 0.000, 0.898)], wheel_r: [0.399, 0.399], min: Vec3::new(-0.876, -0.244, -1.356), max: Vec3::new(0.876, 2.127, 1.356), roof_y: 2.127 },
    CarFit { id: "golf", wheels: [Vec3::new(-0.553, 0.000, -0.993), Vec3::new(0.553, 0.000, -0.993), Vec3::new(-0.553, 0.000, 0.921), Vec3::new(0.553, 0.000, 0.921)], wheel_r: [0.435, 0.435], min: Vec3::new(-0.975, -0.218, -1.849), max: Vec3::new(0.975, 1.450, 1.849), roof_y: 1.450 },
    CarFit { id: "hearse", wheels: [Vec3::new(-0.617, 0.000, -1.552), Vec3::new(0.617, 0.000, -1.552), Vec3::new(-0.617, 0.000, 1.327), Vec3::new(0.617, 0.000, 1.327)], wheel_r: [0.450, 0.450], min: Vec3::new(-1.028, -0.150, -2.437), max: Vec3::new(1.028, 2.250, 2.437), roof_y: 2.250 },
    CarFit { id: "bison", wheels: [Vec3::new(-0.574, 0.000, -1.434), Vec3::new(0.574, 0.000, -1.434), Vec3::new(-0.574, 0.000, 1.191), Vec3::new(0.574, 0.000, 1.191)], wheel_r: [0.486, 0.486], min: Vec3::new(-1.013, -0.243, -2.389), max: Vec3::new(1.013, 1.620, 2.389), roof_y: 1.620 },
    CarFit { id: "wasp", wheels: [Vec3::new(-0.810, 0.000, -1.088), Vec3::new(0.810, 0.000, -1.088), Vec3::new(-0.810, 0.000, 1.496), Vec3::new(0.810, 0.000, 1.496)], wheel_r: [0.510, 0.510], min: Vec3::new(-0.972, -0.340, -2.176), max: Vec3::new(0.972, 0.735, 2.176), roof_y: 0.735 },
    CarFit { id: "limo", wheels: [Vec3::new(-0.582, 0.000, -2.303), Vec3::new(0.582, 0.000, -2.303), Vec3::new(-0.582, 0.000, 2.233), Vec3::new(0.582, 0.000, 2.233)], wheel_r: [0.420, 0.420], min: Vec3::new(-1.028, -0.245, -3.199), max: Vec3::new(1.028, 1.400, 3.199), roof_y: 1.400 },
    CarFit { id: "monster", wheels: [Vec3::new(-0.653, 0.000, -1.573), Vec3::new(0.653, 0.000, -1.573), Vec3::new(-0.653, 0.000, 0.776), Vec3::new(0.653, 0.000, 0.776)], wheel_r: [0.435, 0.435], min: Vec3::new(-1.160, -0.254, -2.501), max: Vec3::new(1.160, 1.885, 2.501), roof_y: 1.885 },
    CarFit { id: "bus", wheels: [Vec3::new(-0.578, 0.000, -1.102), Vec3::new(0.578, 0.000, -1.102), Vec3::new(-0.578, 0.000, 0.812), Vec3::new(0.578, 0.000, 0.812)], wheel_r: [0.435, 0.435], min: Vec3::new(-1.020, -0.145, -1.957), max: Vec3::new(1.020, 1.450, 1.957), roof_y: 1.450 },
    CarFit { id: "roller", wheels: [Vec3::new(-0.845, -0.135, -0.660), Vec3::new(0.845, -0.135, -0.660), Vec3::new(-1.035, 0.135, 1.098), Vec3::new(1.035, 0.135, 1.098)], wheel_r: [0.431, 0.700], min: Vec3::new(-1.145, -0.565, -1.976), max: Vec3::new(1.145, 1.854, 1.976), roof_y: 1.854 },
    CarFit { id: "mixer", wheels: [Vec3::new(-0.747, 0.000, -2.852), Vec3::new(0.747, 0.000, -2.852), Vec3::new(-0.747, 0.000, 2.620), Vec3::new(0.747, 0.000, 2.620)], wheel_r: [0.480, 0.480], min: Vec3::new(-1.245, -0.240, -3.796), max: Vec3::new(1.245, 1.600, 3.796), roof_y: 0.800 },
];

pub fn fit(id: &str) -> Option<&'static CarFit> {
    FITS.iter().find(|f| f.id == id)
}
