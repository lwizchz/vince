/*!
The `Oscillator` module outputs a generated wave with a given gain.

## Wave Functions
 * `Sine` - a smooth wave, the default
 * `Triangle` - a linear sine-like wave
 * `Square` - a binary wave
 * `Saw` - a linear ramp-like wave

## Sync Modes
 * `None` - Do not perform syncing, the default
 * `Horizontal` - Apply horizontal syncing, i.e. reset the phase every video
   frame
 * `Vertical` - Apply vertical syncing, i.e. reset the phase every video line

## Inputs
None

## Outputs
0. The wave signal in the range [-K2, K2] where K2 is knob 2

## Knobs
0. Shift in the range (-inf, inf), affects the signal vertically
1. Speed in the range (-inf, inf), equivalent to the period
2. Depth in the range [0.0, inf), equivalent to the gain
3. Phase in the range (-inf, inf), affects the signal horizontally

*/

use std::f64::consts::PI;

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, component_video_out::ComponentVideoOut}};

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
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    func: OscillatorFunc,
    #[serde(default)]
    sync: OscillatorSync,
    #[serde(skip)]
    sync_phase: f64,
    #[serde(skip)]
    sync_count: usize,

    knobs: [f32; 4],
}
#[typetag::deserialize]
impl Module for Oscillator {
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
                    None => format!("M{id} Oscillator\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["Func\n", "Sync\n", "K0\n", "K1\n", "K2\n", "K3\n"] {
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

    fn step(&mut self, time: f64, _st: StepType, _ins: &[f32]) -> Vec<f32> {
        let t = time;
        let shift = f64::from(self.knobs[0]);
        let speed = f64::from(self.knobs[1]);
        let depth = f64::from(self.knobs[2]);

        match self.sync {
            OscillatorSync::None => {
                self.sync_phase = 0.0;
                self.sync_count = 0;
            },
            OscillatorSync::Horizontal => { // Reset every frame
                if self.sync_count % (ComponentVideoOut::WIDTH * ComponentVideoOut::HEIGHT) == 0 {
                    self.sync_phase = match self.func {
                        OscillatorFunc::Saw => t,
                        _ => speed * t * 2.0*PI,
                    };
                    self.sync_count = 0;
                }
            },
            OscillatorSync::Vertical => { // Reset every line
                if self.sync_count % ComponentVideoOut::WIDTH == 0 {
                    self.sync_phase = match self.func {
                        OscillatorFunc::Saw => t,
                        _ => speed * t * 2.0*PI,
                    };
                    self.sync_count = 0;
                }
            },
        }
        let phase = f64::from(self.knobs[3]) + self.sync_phase;

        let val = match self.func {
            OscillatorFunc::Sine => (speed * t * 2.0*PI - phase).sin() * depth + shift,
            OscillatorFunc::Triangle => 2.0 / PI * depth * ((speed * t * 2.0*PI - phase).sin()).asin() + shift,
            OscillatorFunc::Square => if (speed * t * 2.0*PI - phase).sin() >= 0.0 { depth+shift } else { -depth+shift },
            OscillatorFunc::Saw => {
                let tp = (t - phase) * speed;
                2.0 * (tp - (0.5 + tp).floor()) * depth + shift
            },
        };

        self.sync_count += 1;

        vec![val as f32]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    format!("Func: {:?}\n", self.func),
                    format!("Sync: {:?}\n", self.sync),
                    format!("K0 Shift: {}\n", self.knobs[0]),
                    format!("K1 Speed: {}\n", self.knobs[1]),
                    format!("K2 Depth: {}\n", self.knobs[2]),
                    format!("K3 Phase: {}\n", self.knobs[3]),
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
