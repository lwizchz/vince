/*!
The `Noise` module outputs a noise signal with a given gain.

## Noise Functions
 * `White` - random data from the [rand] crate
 * <strike>`Fractional(f32)` - white noise with a fractional frequency spectrum
   </strike> Not yet supported
 * `Perlin` - smoothed 1-dimensional Perlin noise
 * `Simplex` - smoothed 1-dimensional Simplex noise

## Inputs
None

## Outputs
0. The noise signal in the range [-K0, K0] where K0 is knob 0

## Knobs
0. Gain in the range [0.0, 1.0]

*/

use rand::prelude::*;

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
enum NoiseFunc {
    #[default]
    White,
    // Fractional(f32),

    Perlin,
    Simplex,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Noise {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(default)]
    func: NoiseFunc,
    knobs: [f32; 1],
}
#[typetag::deserialize]
impl Module for Noise {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Relative,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} Noise\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("Func:\n".to_string(), ts.clone()),
                            TextSection::new("F\n".to_string(), ts.clone()),
                            TextSection::new("K0\n".to_string(), ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        0
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        1
    }

    fn step(&mut self, time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![0.0];
        }

        match self.func {
            NoiseFunc::White => vec![thread_rng().gen_range(-1.0..=1.0) * self.knobs[0]],
            // NoiseFunc::Fractional(_p) => {
            //     // FIXME actually do this
            //     vec![thread_rng().gen_range(-1.0..=1.0) * self.knobs[0]]
            // },

            NoiseFunc::Perlin => vec![perlin(time) as f32 * self.knobs[0]],
            NoiseFunc::Simplex => vec![simplex(time) as f32 * self.knobs[0]],
        }
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[2].value = format!("{:?}\n", self.func);
                text.sections[3].value = format!("K0 Level: {}\n", self.knobs[0]);
            }
        }
    }
}

const PERM: [i32; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36,
    103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0,
    26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56,
    87, 174, 20, 125, 136, 171, 168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166,
    77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55,
    46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132,
    187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109,
    198, 173, 186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126,
    255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183,
    170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43,
    172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112,
    104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106,
    157, 184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205,
    93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
];
fn perlin(mut x: f64) -> f64 {
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
    fn grad(hash: i32, x: f64) -> f64 {
        if hash & 1 == 1 {
            x
        } else {
            -x
        }
    }
    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b-a)
    }

    let p: Vec<i32> = PERM.iter()
        .chain(PERM.iter())
        .copied()
        .collect();

    let xint = x.floor() as usize & 255;
    x -= x.floor();
    let u = fade(x);

    lerp(
        u,
        grad(p[xint], x),
        grad(p[xint+1], x-1.0),
    )
}
fn simplex(x: f64) -> f64 {
    fn grad(hash: i32, x: f64) -> f64 {
        let h = hash & 15;
        let g = 1.0 + (h & 7) as f64;
        if h & 8 == 0 {
            g * x
        } else {
            -g * x
        }
    }

    let p: Vec<i32> = PERM.iter()
        .chain(PERM.iter())
        .copied()
        .collect();

    let i0 = x.floor() as usize;
    let i1 = i0 + 1;
    let x0 = x - i0 as f64;
    let x1 = x0 - 1.0;

    let t0 = (1.0 - x0 * x0).powi(2);
    let n0 = t0 * t0 * grad(p[i0 & 255], x0);

    let t1 = (1.0 - x1 * x1).powi(2);
    let n1 = t1 * t1 * grad(p[i1 & 255], x1);

    0.395 * (n0+n1)
}
