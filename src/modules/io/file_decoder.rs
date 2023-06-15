/*!
The `FileDecoder` module takes a WAV file and outputs it, looping upon reaching
the end.

## Inputs
None

## Outputs
0. The left channel of the audio signal
1. The right channel of the audio signal

##### Note
If the right channel is missing, then the left channel will be copied to both
outputs.

##### Note
If the audio buffer becomes empty, the outputs will both be [f32::NAN].

## Knobs
0. Gain in the range [0.0, inf)

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

pub(crate) struct FileReader {
    filename: String,
    reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
    idx: usize,
    buffer: Vec<[f32; 2]>,
}
impl FileReader {
    pub(crate) fn new(filename: &str) -> Self {
        FileReader {
            filename: filename.to_string(),
            reader: hound::WavReader::open(filename)
                .unwrap_or_else(|msg| panic!("Failed to open WAV file {}: {}", filename, msg)),
            idx: 0,
            buffer: vec![],
        }
    }
    fn append_sample(&mut self, s: Result<i16, hound::Error>) {
        let left = s
            .unwrap_or_else(|msg| panic!("Failed to read sample from WAV file {}: {}", self.filename, msg))
            as f32 / i16::MAX as f32;
        let right = if self.reader.spec().channels == 1 {
            left
        } else {
            self.reader.samples::<i16>().next()
                .unwrap_or_else(|| panic!("Failed to continue reading channel sample from WAV file {}: unbalanced stream", self.filename))
                .unwrap_or_else(|msg| panic!("Failed to continue reading channel sample from WAV file {}: {}", self.filename, msg))
                as f32 / i16::MAX as f32
        };

        self.buffer.push([left, right]);
        self.idx += 1;
    }
    pub(crate) fn rewind(&mut self) {
        // Finish reading before rewinding
        while let Some(s) = self.reader.samples::<i16>().next() {
            self.append_sample(s);
        }

        self.idx = 0;
    }
    pub(crate) fn read_sample(&mut self, should_loop: bool) -> Option<[f32; 2]> {
        let sample = self.reader.samples::<i16>().next();
        match sample {
            Some(s) => {
                self.append_sample(s);
            },
            None => {
                self.idx += 1;
                if should_loop {
                    self.idx %= self.buffer.len();
                } else if self.idx-1 == self.buffer.len() {
                    return None;
                }
            }
        }
        if self.idx > 0 {
            Some(self.buffer[self.idx-1])
        } else {
            Some(self.buffer[self.buffer.len() - 1])
        }
    }
}
impl std::fmt::Debug for FileReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileReader {{ filename: \"{}\" }}", self.filename)
    }
}
impl Clone for FileReader {
    fn clone(&self) -> Self {
        FileReader::new(&self.filename)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileDecoder {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    reader: Option<FileReader>,

    filename: String,
    knobs: [f32; 1],
}
impl FileDecoder {
    pub fn new(filename: &str, gain: f32) -> Self {
        Self {
            id: None,
            name: None,

            component: None,
            children: vec![],

            reader: None,

            filename: filename.to_string(),
            knobs: [gain],
        }
    }
}
#[typetag::deserialize]
impl Module for FileDecoder {
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
                    None => format!("M{id} File Decoder\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new(format!("{}\n", self.filename), ts.clone()),
                            TextSection::new("K0", ts),
                        ]).with_style(Style {
                            size: Size {
                                width: Val::Px(150.0),
                                height: Val::Px(180.0),
                            },
                            flex_wrap: FlexWrap::Wrap,
                            ..default()
                        }),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        self.reader = Some(FileReader::new(&self.filename));
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
        2
    }
    fn knobs(&self) -> usize {
        1
    }

    fn step(&mut self, _time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![f32::NAN, f32::NAN];
        }

        if let Some(reader) = &mut self.reader {
            let sample = reader.read_sample(true).unwrap();
            vec![
                sample[0] * self.knobs[0],
                sample[1] * self.knobs[0],
            ]
        } else {
            vec![f32::NAN, f32::NAN]
        }
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[2].value = format!("K0 Gain: {}\n", self.knobs[0]);
            }
        }
    }
}
