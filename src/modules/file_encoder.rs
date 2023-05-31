/*!
The `FileEncoder` module takes 2 inputs and writes them as stereo to a WAV
file.

## Inputs
0. The left channel of the audio signal
1. The right channel of the audio signal

##### Note
If the right channel is NAN (unpatched), then the left channel will be doubled.

## Outputs
None

## Knobs
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent}};

struct FileWriter {
    filename: String,
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
}
impl FileWriter {
    fn new(filename: &str) -> Self {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        FileWriter {
            filename: filename.to_string(),
            writer: hound::WavWriter::create(filename, spec).expect(&format!("Failed to create WAV file: {}", filename)),
        }
    }
}
impl std::fmt::Debug for FileWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", "FileWriter { filename: \"", self.filename, "\" }")
    }
}
impl Clone for FileWriter {
    fn clone(&self) -> Self {
        FileWriter::new(&self.filename)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileEncoder {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    writer: Option<FileWriter>,

    filename: String,
}
#[typetag::deserialize]
impl Module for FileEncoder {
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
                    None => format!("M{id} File Encoder\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new(
                                format!(
                                    "{}\n",
                                    self.filename.chars()
                                        .enumerate()
                                        .flat_map(|(i, c)| {
                                            if i > 0 && i % 17 == 0 {
                                                vec![c, '\n']
                                            } else {
                                                vec![c]
                                            }
                                        }).collect::<String>()
                                        .trim_end(),
                                ),
                                ts,
                            ),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        self.writer = Some(FileWriter::new(&self.filename));
    }
    fn exit(&mut self) {
        if let Some(writer) = self.writer.take() {
            writer.writer.finalize().unwrap();
        }
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        2
    }
    fn outputs(&self) -> usize {
        0
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, st: StepType, ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![];
        }

        let left = (ins[0] * i16::MAX as f32) as i16;
        let right = if ins[1].is_nan() {
            left
        } else {
            (ins[1] * i16::MAX as f32) as i16
        };

        if let Some(writer) = &mut self.writer {
            writer.writer.write_sample(left).unwrap();
            writer.writer.write_sample(right).unwrap();
        } else {
            error!("FileEncoder dropped audio output");
        }

        vec![]
    }
}
