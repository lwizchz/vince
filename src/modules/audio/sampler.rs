/*!
The `Sampler` module outputs signals from the given samples according to the
associated sequence.

## Samples
Each sample is represented by a subarray whose elements are the sample's
filename and an array representing the sequence for that sample. Each sequence
element is a subarray that consists of the beat index and the volume. The beat
index is in terms of quarter notes where 1.0 represents the length of a single
quarter note.

## Inputs
None

## Outputs
0. The signal from the first sample
1. The signal from the second sample
...
N. The signal from the Nth sample

## Knobs
0. Tempo in the range (0.0, inf)
1. Sequence length in the range (0.0, inf) in beats

*/

use std::path::Path;

use bevy::{prelude::*, ecs::system::EntityCommands, utils::HashMap};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, io::file_decoder::{FileReader, WavReader}}};

#[derive(Deserialize, Debug, Clone)]
pub struct Sampler {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    pub(crate) samples: Vec<(String, Vec<(f32, f32)>)>,
    #[serde(skip)]
    sample_readers: Vec<FileReader>,
    #[serde(skip)]
    active_samples: HashMap<usize, f32>,
    #[serde(skip)]
    pub(crate) time: f64,
    #[serde(skip)]
    pub(crate) last_time: Option<f64>,

    knobs: [f32; 2],
}
impl Sampler {
    pub(crate) fn init_readers(&mut self) {
        self.sample_readers = self.samples.iter()
            .map(|(filename, _)| {
                let mut reader = FileReader::WavReader(WavReader::new(filename));
                reader.rewind();
                reader
            }).collect();
    }
}
#[typetag::deserialize]
impl Module for Sampler {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor)) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(150.0),
                    height: Val::Px(180.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} Sampler\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["K0\n", "K1\n", "Beat\n", "Active\n"] {
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

        self.init_readers();
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
        self.samples.len()
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

    fn step(&mut self, time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![f32::NAN; self.outputs()];
        }

        if self.sample_readers.is_empty() {
            self.init_readers();
        }

        const EPSILON: f64 = 0.0625;

        let tempo = self.knobs[0];
        let length = self.knobs[1];

        self.time += time - self.last_time.unwrap_or(time);
        self.time %= length as f64 * 60.0 / tempo as f64;
        self.last_time = Some(time);

        let beat = self.time * tempo as f64 / 60.0;

        let mut outs = Vec::with_capacity(self.outputs());
        for (i, (_, seq)) in self.samples.iter()
            .enumerate()
        {
            let reader = &mut self.sample_readers[i];
            for (b, v) in seq {
                if (beat - *b as f64).abs() < EPSILON {
                    reader.rewind();
                    self.active_samples.insert(i, *v);
                    break;
                }
            }
            match self.active_samples.get(&i) {
                Some(v) => {
                    match reader.read_sample(false) {
                        Some(sample) => {
                            outs.push((sample[0] + sample[1]) / 2.0 * v);
                        },
                        None => {
                            outs.push(0.0);
                            self.active_samples.remove(&i);
                        },
                    }
                },
                None => outs.push(0.0),
            }
        }

        outs
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let mut texts = vec![];

            texts.push(format!("K0 Tempo: {}\n", self.knobs[0]));
            texts.push(format!("K1 Length: {}\n", self.knobs[1]));

            let tempo = self.knobs[0];
            let beat = self.time * tempo as f64 / 60.0;
            texts.push(format!("Beat: {}\n", beat.floor() as usize + 1));

            if self.active_samples.is_empty() {
                texts.push("Active: None\n".to_string());
            } else {
                let active = self.active_samples.iter()
                    .map(|(sidx, _)| {
                        match Path::new(&self.samples[*sidx].0).file_prefix() {
                            Some(fp) => fp.to_string_lossy().to_string(),
                            None => format!("SAMP{sidx}"),
                        }
                    }).fold(String::new(), |mut acc, s| {
                        acc += &s;
                        acc += " ";
                        acc
                    });
                texts.push(format!("Active: {}\n", active));
            }

            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip(texts).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
