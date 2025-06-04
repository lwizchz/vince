/*!
The `Delay` module takes an input and applies a delay to it.

## Inputs
0. The signal to delay

## Outputs
0. The delayed signal

## Knobs
0. Delay in the range (0.0, inf) in seconds
1. Feedback in the range [0.0, 1.0]
2. Dry/Wet mix in the range [0.0, 1.0]

*/

use std::cmp::Ordering;

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct Delay {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    delay_idx: usize,
    #[serde(skip)]
    buffer: Vec<f32>,

    knobs: [f32; 3],
}
#[typetag::deserialize]
impl Module for Delay {
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
                    None => format!("M{id} Delay\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["K0\n", "K1\n", "K2\n"] {
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
    }

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let x = ins[0];
        if x.is_nan() {
            return vec![f32::NAN];
        }

        let delay = self.knobs[0];
        let feedback = self.knobs[1];
        let dwmix = self.knobs[2];

        let sr = 44100.0;
        let buflen = (delay * sr) as usize;
        match self.buffer.len().cmp(&buflen) {
            Ordering::Less => {
                self.buffer.extend(vec![0.0; buflen - self.buffer.len()]);
            },
            Ordering::Greater => {
                self.buffer.drain(buflen..);
            },
            Ordering::Equal => {},
        }

        let delayed = if buflen > 0 {
            self.delay_idx %= buflen;
            let delayed = self.buffer[self.delay_idx];
            self.buffer[self.delay_idx] = feedback * (x + delayed);
            delayed
        } else {
            0.0
        };

        self.delay_idx += 1;

        vec![
            x * (1.0 - dwmix)
            + delayed * dwmix
        ]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    format!("K0 Delay: {}\n", self.knobs[0]),
                    format!("K1 Feedback: {}\n", self.knobs[1]),
                    format!("K2 Dry/Wet: {}\n", self.knobs[2]),
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
