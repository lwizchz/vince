/*!
The `Equalizer` module takes an input and applies a gain to it in the given
frequency range.

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

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

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

    #[serde(skip)]
    xs: [f32; 2],
    #[serde(skip)]
    ys: [f32; 2],

    knobs: [f32; 3],
}
#[typetag::deserialize]
impl Module for Equalizer {
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
                    None => format!("M{id} Equalizer\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("K0\n".to_string(), ts.clone()),
                            TextSection::new("K1\n".to_string(), ts.clone()),
                            TextSection::new("K2\n".to_string(), ts),
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

        // bpf
        let a = [
            1.0 + alpha,
            -2.0 * w_0.cos(),
            1.0 - alpha,
        ];
        let b = [
            g * w_0.sin() / 2.0,
            0.0,
            g * -w_0.sin() / 2.0,
        ];

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
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Frequency: {}\n", self.knobs[0]);
                text.sections[2].value = format!("K1 Q: {}\n", self.knobs[1]);
                text.sections[3].value = format!("K2 Gain: {}\n", self.knobs[2]);
            }
        }
    }
}
