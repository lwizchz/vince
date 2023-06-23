/*!
The `PitchShifter` module takes an input and outputs a pitch shifted version of
it.

## Pitch Shift Functions
 * `Simple` - Shifts all frequencies by the given amount, the default
 * `PhaseVocoder` - Uses the phase vocoder algorithm to process frequencies
   more precisely

## Inputs
0. The signal to pitch shift

## Outputs
0. The pitch shifted signal
1. The frequency of the primary detected pitch
2. The frequency of the secondary detected pitch
3. The frequency of the tertiary detected pitch

##### Note
The outputs will be [f32::NAN] for the first [PitchShifter::BUFSIZE] frames
while the FFT buffer is being populated.

## Knobs
0. Pitch shift amount in the range (-inf, inf) in semitones

*/

use std::{cmp::Ordering, f32::consts::PI};

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use rustfft::{FftPlanner, num_complex::Complex};

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
enum PitchShifterFunc {
    #[default]
    Simple,
    PhaseVocoder,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PitchShifter {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    in_buffer: Vec<Complex<f32>>,
    #[serde(skip)]
    out_buffer: Vec<Complex<f32>>,
    #[serde(skip)]
    window: Vec<f32>,
    #[serde(skip)]
    bin_energies: Vec<(usize, f32)>,

    #[serde(default)]
    func: PitchShifterFunc,

    knobs: [f32; 1],
}
impl PitchShifter {
    const BUFSIZE: usize = 4096;
}
#[typetag::deserialize]
impl Module for PitchShifter {
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
                    None => format!("M{id} PitchShifter\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("Func\n", ts.clone()),
                            TextSection::new("K0\n", ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        self.in_buffer = Vec::with_capacity(PitchShifter::BUFSIZE);

        // Generate Gaussian window
        const SIGMA: f32 = 0.4;
        self.window = (0..PitchShifter::BUFSIZE)
            .map(|n| {
                (-0.5 * ((n as f32 - PitchShifter::BUFSIZE as f32 / 2.0) / (SIGMA * PitchShifter::BUFSIZE as f32 / 2.0)).powi(2)).exp()
            }).collect();
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
        4
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
        let shift = self.knobs[0];

        const SR: f32 = 44100.0;

        let x = ins[0];
        if self.in_buffer.len() < PitchShifter::BUFSIZE {
            self.in_buffer.push(Complex { re: x, im: 0.0 });
            let out = if !self.out_buffer.is_empty() {
                vec![
                    self.out_buffer.remove(0).re,
                    self.bin_energies[0].0 as f32 * SR / PitchShifter::BUFSIZE as f32,
                    self.bin_energies[1].0 as f32 * SR / PitchShifter::BUFSIZE as f32,
                    self.bin_energies[2].0 as f32 * SR / PitchShifter::BUFSIZE as f32,
                ]
            } else {
                vec![f32::NAN; self.outputs()]
            };

            if self.in_buffer.len() == PitchShifter::BUFSIZE {
                let mut planner = FftPlanner::new();
                let fft = planner.plan_fft_forward(PitchShifter::BUFSIZE);
                self.out_buffer = self.in_buffer.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        c * self.window[i]
                    }).collect();
                self.in_buffer.drain(..PitchShifter::BUFSIZE / 2); // 50% overlap
                fft.process(&mut self.out_buffer);

                self.bin_energies = self.out_buffer.iter()
                    .copied()
                    .enumerate()
                    .skip(1)
                    .take(PitchShifter::BUFSIZE / 2)
                    .map(|(i, v)| (i, v.norm()))
                    .collect();
                self.bin_energies.sort_by(|(_, a), (_, b)| {
                    a.partial_cmp(&b)
                        .unwrap_or_else(|| {
                            if a.is_nan() && b.is_nan() {
                                Ordering::Equal
                            } else if a.is_nan() {
                                Ordering::Less
                            } else if b.is_nan() {
                                Ordering::Greater
                            } else {
                                panic!("Failed to determine PitchShifter primary bin: uncomparable values {a} and {b}");
                            }
                        })
                });
                self.bin_energies.reverse();

                if shift != 0.0 {
                    let mut out_buffer = self.out_buffer.drain(..)
                        .collect::<Vec<Complex<f32>>>();
                    self.out_buffer = vec![Complex { re: 0.0, im: 0.0 }; PitchShifter::BUFSIZE];
                    for (bin, v) in out_buffer.drain(..)
                        .enumerate()
                    {
                        let shifted_bin = (bin as f32 * 2.0f32.powf(shift / 12.0)).floor();
                        if shifted_bin > 0.0 && shifted_bin < self.out_buffer.len() as f32 {
                            self.out_buffer[shifted_bin as usize] = match self.func {
                                PitchShifterFunc::Simple => {
                                    v
                                },
                                PitchShifterFunc::PhaseVocoder => {
                                    let omega_delta = 2.0 * PI * (shifted_bin - bin as f32)/ PitchShifter::BUFSIZE as f32;
                                    let phase_shift = Complex {
                                        re: (omega_delta * time as f32).cos(),
                                        im: (omega_delta * time as f32).sin(),
                                    };

                                    Complex {
                                        re: v.re * phase_shift.re - v.im * phase_shift.im,
                                        im: v.re * phase_shift.im + v.im * phase_shift.re,
                                    }
                                },
                            };
                        }
                    }
                }

                let ifft = planner.plan_fft_inverse(PitchShifter::BUFSIZE);
                ifft.process(&mut self.out_buffer);

                for o in &mut self.out_buffer {
                    *o /= PitchShifter::BUFSIZE as f32;
                }
            }

            return out;
        }

        unreachable!();
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("Func: {:?}\n", self.func);
                text.sections[2].value = format!("K0 Shift: {}\n", self.knobs[0]);
            }
        }
    }
}
