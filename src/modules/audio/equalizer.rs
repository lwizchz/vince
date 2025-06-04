/*!
The `Equalizer` module takes an input and applies a gain to it in the given
frequency range based on the chosen filter function.

## Filter Functions
 * `LPF` - a low-pass filter
 * `HPF` - a high-pass filter
 * `BPF` - a band-pass filter, the default
 * `Notch` - a notch filter
 * `APF` - an all-pass filter
 * `LowShelf` - a low shelf
 * `HighShelf` - a high shelf

## Inputs
0. The signal to boost or cut

## Outputs
0. The resulting boosted or cut signal

## Knobs
0. Frequency to boost or cut in the range (0.0, inf)
1. Q in the range (0.0, inf)
2. Gain in the range [0.0, inf)

*/

use std::f32::consts::PI;

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
pub enum EqualizerFunc {
    LPF,
    HPF,
    #[default]
    BPF,
    Notch,
    APF,
    LowShelf,
    HighShelf,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Equalizer {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(default)]
    func: EqualizerFunc,

    #[serde(skip)]
    xs: [f32; 2],
    #[serde(skip)]
    ys: [f32; 2],

    knobs: [f32; 3],
}
#[typetag::deserialize]
impl Module for Equalizer {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor)) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} Equalizer\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["Func\n", "K0\n", "K1\n", "K2\n"] {
                            p.spawn((
                                TextSpan::new(t),
                                tfc.0.clone(),
                                tfc.1.clone(),
                            ));
                        }
                    }).id()
                );
            });
            self.component = Some(component.id());
        });
    }
    fn exit(&mut self) {
        self.id = None;
        self.component = None;
        self.children = vec![];
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        1
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        self.knobs.len()
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
        if i == 0 {
            // Reset Y's to prevent runaway feedback
            self.ys = [0.0, 0.0];
        }
    }

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let x = ins[0];
        if x.is_nan() {
            return vec![f32::NAN];
        }

        let f_0 = self.knobs[0];
        let q = self.knobs[1];
        let g = self.knobs[2];

        let f_s = 44100.0;

        let w_0 = 2.0 * PI * f_0 / f_s;
        let alpha = w_0.sin() / 2.0 / q;

        let (a, b) = match self.func {
            EqualizerFunc::LPF => (
                [
                    1.0 + alpha,
                    -2.0 * w_0.cos(),
                    1.0 - alpha,
                ],
                [
                    g * (1.0 - w_0.cos()) / 2.0,
                    g * (1.0 - w_0.cos()),
                    g * (1.0 - w_0.cos()) / 2.0,
                ]
            ),
            EqualizerFunc::HPF => (
                [
                    1.0 + alpha,
                    -2.0 * w_0.cos(),
                    1.0 - alpha,
                ],
                [
                    g * (1.0 + w_0.cos()) / 2.0,
                    g * (-1.0 - w_0.cos()),
                    g * (1.0 + w_0.cos()) / 2.0,
                ]
            ),
            EqualizerFunc::BPF => (
                [
                    1.0 + alpha,
                    -2.0 * w_0.cos(),
                    1.0 - alpha,
                ],
                [
                    g * w_0.sin() / 2.0,
                    0.0,
                    g * -w_0.sin() / 2.0,
                ]
            ),
            EqualizerFunc::Notch => (
                [
                    1.0 + alpha,
                    -2.0 * w_0.cos(),
                    1.0 - alpha,
                ],
                [
                    g,
                    g * -2.0 * w_0.cos(),
                    g,
                ]
            ),
            EqualizerFunc::APF => (
                [
                    1.0 + alpha,
                    -2.0 * w_0.cos(),
                    1.0 - alpha,
                ],
                [
                    g * (1.0 - alpha),
                    g * -2.0 * w_0.cos(),
                    g * (1.0 + alpha),
                ]
            ),
            EqualizerFunc::LowShelf => {
                let dbgain = 10.0 * g.log10();
                let big_a = 10.0f32.powf(dbgain / 40.0);
                (
                    [
                        big_a + 1.0 + (big_a - 1.0) * w_0.cos() + 2.0 * big_a.sqrt() * alpha,
                        -2.0 * (big_a - 1.0 + (big_a + 1.0) * w_0.cos()),
                        big_a + 1.0 + (big_a - 1.0) * w_0.cos() - 2.0 * big_a.sqrt() * alpha,
                    ],
                    [
                        big_a * (big_a + 1.0 - (big_a  - 1.0) * w_0.cos() + 2.0 * big_a.sqrt() * alpha),
                        2.0 * big_a * (big_a - 1.0 - (big_a + 1.0) * w_0.cos()),
                        big_a * (big_a + 1.0 - (big_a - 1.0) * w_0.cos() - 2.0 * big_a.sqrt() * alpha),
                    ]
                )
            },
            EqualizerFunc::HighShelf => {
                let dbgain = 10.0 * g.log10();
                let big_a = 10.0f32.powf(dbgain / 40.0);
                (
                    [
                        big_a + 1.0 - (big_a - 1.0) * w_0.cos() + 2.0 * big_a.sqrt() * alpha,
                        2.0 * (big_a - 1.0 - (big_a + 1.0) * w_0.cos()),
                        big_a + 1.0 - (big_a - 1.0) * w_0.cos() - 2.0 * big_a.sqrt() * alpha,
                    ],
                    [
                        big_a * (big_a + 1.0 + (big_a  - 1.0) * w_0.cos() + 2.0 * big_a.sqrt() * alpha),
                        -2.0 * big_a * (big_a - 1.0 + (big_a + 1.0) * w_0.cos()),
                        big_a * (big_a + 1.0 + (big_a - 1.0) * w_0.cos() - 2.0 * big_a.sqrt() * alpha),
                    ]
                )
            },
        };

        let y = b[0] / a[0] * x
            + b[1] / a[0] * self.xs[0]
            + b[2] / a[0] * self.xs[1]
            - a[1] / a[0] * self.ys[0]
            - a[2] / a[0] * self.ys[1];

        self.ys[1] = self.ys[0];
        self.ys[0] = y;
        self.xs[1] = self.xs[0];
        self.xs[0] = x;

        vec![y]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    format!("Func: {:?}\n", self.func),
                    format!("K0 Frequency: {}\n", self.knobs[0]),
                    format!("K1 Q: {}\n", self.knobs[1]),
                    format!("K2 Gain: {}\n", self.knobs[2]),
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
