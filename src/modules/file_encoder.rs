/*!
The `FileEncoder` module takes an input and writes it to a WAV file.

## Inputs
0. The audio signal to write (stereo is not currently supported)

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
            channels: 1,
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
                            TextSection::new(self.filename.clone(), ts),
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
        1
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

        if let Some(writer) = &mut self.writer {
            writer.writer.write_sample((ins[0] * i16::MAX as f32) as i16).unwrap();
        } else {
            error!("FileEncoder dropped audio output");
        }

        vec![]
    }
}
