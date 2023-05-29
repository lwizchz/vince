use std::f32::consts::PI;

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, component_video_out::ComponentVideoOut}};

const fn _default_true() -> bool {
    true
}

#[derive(Default, Deserialize, Debug, Clone)]
enum OscillatorFunc {
    #[default]
    Sine,
    Triangle,
    Square,
    Saw,
}
#[derive(Default, Deserialize, Debug, Clone)]
enum OscillatorSync {
    #[default]
    None,
    Horizontal,
    Vertical,
}
#[derive(Default, Deserialize, Debug, Clone)]
pub struct Oscillator {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    #[serde(default)]
    sync: OscillatorSync,
    #[serde(default)]
    sync_phase: f32,
    #[serde(default)]
    sync_count: usize,

    func: OscillatorFunc,
    knobs: [f32; 4],
}
#[typetag::deserialize]
impl Module for Oscillator {
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
                    None => format!("M{id} Oscillator\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("Func\n".to_string(), ts.clone()),
                            TextSection::new("Sync\n".to_string(), ts.clone()),
                            TextSection::new("K0\n".to_string(), ts.clone()),
                            TextSection::new("K1\n".to_string(), ts.clone()),
                            TextSection::new("K2\n".to_string(), ts.clone()),
                            TextSection::new("K3\n".to_string(), ts),
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
        self.knobs.len()
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
    }

    fn step(&mut self, time: f32, _ft: StepType, _ins: &[f32]) -> Vec<f32> {
        let t = time;
        let shift = self.knobs[0];
        let speed = self.knobs[1];
        let depth = self.knobs[2];

        // FIXME video sync
        match self.sync {
            OscillatorSync::None => self.sync_phase = 0.0,
            OscillatorSync::Horizontal => { // Reset every frame
                if self.sync_count % (ComponentVideoOut::WIDTH as usize * ComponentVideoOut::HEIGHT as usize) == 0 {
                    self.sync_phase = match self.func {
                        OscillatorFunc::Saw => t,
                        _ => speed * t,
                    };
                    self.sync_count = 0;
                }
            },
            OscillatorSync::Vertical => { // Reset every line
                if self.sync_count % (ComponentVideoOut::WIDTH as usize) == 0 {
                    self.sync_phase = match self.func {
                        OscillatorFunc::Saw => t,
                        _ => speed * t,
                    };
                    self.sync_count = 0;
                }
            },
        }
        let phase = self.knobs[3] + self.sync_phase;

        let val = match self.func {
            OscillatorFunc::Sine => (speed * t - phase).sin() * depth / 2.0 + shift,
            OscillatorFunc::Triangle => 1.0 / PI * depth * ((speed * t - phase).sin()).asin() + shift,
            OscillatorFunc::Square => if (speed * t - phase).sin() >= 0.0 { depth/2.0+shift } else { -depth/2.0+shift },
            OscillatorFunc::Saw => {
                let tp = (t - phase) * speed / 2.0 / PI;
                (tp - (0.5 + tp).floor()) * depth + shift
            },
        };

        self.sync_count += 1;

        vec![val]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("Func: {:?}\n", self.func);
                text.sections[2].value = format!("Sync: {:?}\n", self.sync);
                text.sections[3].value = format!("K0 Shift: {}\n", self.knobs[0]);
                text.sections[4].value = format!("K1 Speed: {}\n", self.knobs[1]);
                text.sections[5].value = format!("K2 Depth: {}\n", self.knobs[2]);
                text.sections[6].value = format!("K3 Phase: {}\n", self.knobs[3]);
            }
        }
    }
}
