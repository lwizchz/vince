/*!
The `EnvelopeGenerator` module outputs an ADSR envelope based on the given
parameters.

## Inputs
0. The envelope's max level
1. The envelope's attack/sustain/release behavior according to the below table:
   * If just triggered this frame: 1.0
   * If just released this frame: -1.0
   * Otherwise: 0.0

## Outputs
0. The envelope's level

## Knobs
0. Attack time in the range [0.0, inf)
1. Decay time in the range [0.0, inf)
2. Sustain level in the range [0.0, inf)
3. Release time in the range [0.0, inf)

*/

use std::f32::EPSILON;

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct EnvelopeGenerator {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    attack_timestamp: Option<f64>,
    #[serde(skip)]
    release_timestamp: Option<f64>,
    #[serde(skip)]
    last_level: Option<f32>,

    knobs: [f32; 4],
}
#[typetag::deserialize]
impl Module for EnvelopeGenerator {
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
                    None => format!("M{id} EnvelopeGenerator\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["K0\n", "K1\n", "K2\n", "K3\n"] {
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
        2
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

    fn step(&mut self, time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let attack = self.knobs[0];
        let decay = self.knobs[1];
        let sustain = self.knobs[2];
        let release = self.knobs[3];

        let x = if ins[0].is_nan() {
            1.0
        } else {
            ins[0]
        };
        let asr = ins[1];
        if asr != 1.0 && asr != 0.0 && asr != -1.0 {
            error!("Invalid attack/sustain/release input value: {asr}");
        }
        let mut y = 0.0f32;

        match self.attack_timestamp {
            Some(at) => {
                match self.release_timestamp {
                    Some(rt) => {
                        if asr == 1.0 {
                            self.attack_timestamp = Some(time);
                            self.release_timestamp = None;
                        } else if asr == -1.0 {
                            error!("Can't release the envelope when it's already been released");
                        } else if asr == 0.0 {
                            let rdt = time - rt;
                            if rdt < release as f64 {
                                let level = if x == 0.0 {
                                    if let Some(last_level) = self.last_level {
                                        last_level as f64 * sustain as f64
                                    } else {
                                        sustain as f64
                                    }
                                } else {
                                    x as f64 * sustain as f64
                                };
                                y = (level - rdt * level / release as f64) as f32;
                            }
                        }
                    },
                    None => {
                        if asr == 1.0 {
                            self.attack_timestamp = Some(time);
                        } else if asr == -1.0 {
                            self.release_timestamp = Some(time);
                        } else if asr == 0.0 {
                            let adt = time - at;
                            if adt < attack as f64 {
                                let level = x as f64;
                                y = (adt * level / attack as f64) as f32;
                            } else if (adt - attack as f64) < decay as f64 {
                                let ddt = adt - attack as f64;
                                let level = x as f64;
                                y = (level + ddt / decay as f64 * (sustain as f64 - level)) as f32;
                                if y < sustain {
                                    y = sustain;
                                }
                            } else {
                                y = sustain;
                            }
                        }
                    },
                }
            },
            None => {
                match self.release_timestamp {
                    Some(_rt) => unreachable!(),
                    None => {
                        if asr == 1.0 {
                            self.attack_timestamp = Some(time);
                        } else if asr == 0.0 {
                            if x > 0.0 && !ins[0].is_nan() {
                                error!("Can't sustain the envelope when it hasn't been triggered");
                            }
                        } else if asr == -1.0 {
                            error!("Can't release the envelope when it hasn't been triggered");
                        }

                        // Leave y = 0.0 since that's where it starts
                    },
                }
            },
        }

        if x != 0.0 {
            self.last_level = Some(x);
        }

        if y.abs() < EPSILON {
            y = 0.0;
        }

        vec![y]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    format!("K0 Attack: {}\n", self.knobs[0]),
                    format!("K1 Decay: {}\n", self.knobs[1]),
                    format!("K2 Sustain: {}\n", self.knobs[2]),
                    format!("K3 Release: {}\n", self.knobs[3]),
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
