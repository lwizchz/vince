/*!
The `FileEncoder` module takes either 2 or 3 inputs and writes them as either
stereo to a WAV file or RGB data to a Y4M file.

## Inputs
 * If writing to a WAV file:
   0. The left channel of the audio signal
   1. The right channel of the audio signal
 * If writing to a Y4M file:
   0. The red channel
   1. The green channel
   2. The blue channel

##### Note
If writing to a WAV file and the right channel is [f32::NAN] (unpatched), then
the left channel will be copied to it.

## Outputs
None

## Knobs
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, component_video_out::ComponentVideoOut}};

struct WavWriter {
    filename: String,
    writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
}
impl WavWriter {
    fn new(filename: &str) -> Self {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        WavWriter {
            filename: filename.to_string(),
            writer: Some(hound::WavWriter::create(filename, spec)
                .unwrap_or_else(|msg| panic!("Failed to create WAV file {}: {}", filename, msg))),
        }
    }
    fn write_sample(&mut self, sample: f32) -> Result<(), hound::Error> {
        let sample = (sample * i16::MAX as f32) as i16;
        if let Some(writer) = &mut self.writer {
            writer.write_sample(sample)
        } else {
            Ok(())
        }
    }
    fn finalize(&mut self) -> Result<(), hound::Error> {
        if let Some(writer) = self.writer.take() {
            writer.finalize()
        } else {
            Ok(())
        }
    }
}
impl std::fmt::Debug for WavWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WavWriter {{ filename: \"{}\" }}", self.filename)
    }
}
impl Clone for WavWriter {
    fn clone(&self) -> Self {
        WavWriter::new(&self.filename)
    }
}

struct Y4mWriter {
    filename: String,
    writer: y4m::Encoder<std::io::BufWriter<std::fs::File>>,

    next_frame: Vec<f32>,
}
impl Y4mWriter {
    fn new(filename: &str) -> Self {
        Y4mWriter {
            filename: filename.to_string(),
            writer: y4m::EncoderBuilder::new(
                ComponentVideoOut::WIDTH,
                ComponentVideoOut::HEIGHT,
                y4m::Ratio {
                    num: 147,
                    den: 4,
                },
            ).with_colorspace(y4m::Colorspace::C420mpeg2)
            .write_header(std::io::BufWriter::new(
                std::fs::File::create(filename)
                    .unwrap_or_else(|e| panic!("Failed to open Y4M file for writing {}: {e}", filename))
            )).unwrap_or_else(|e| panic!("Failed to write Y4M file header {}: {e}", filename)),

            next_frame: vec![],
        }
    }
    fn write_sample(&mut self, sample: f32) -> Result<(), y4m::Error> {
        if self.next_frame.len() == ComponentVideoOut::WIDTH * ComponentVideoOut::HEIGHT * 3 {
            let ys = self.next_frame.chunks(3)
                .map(|rgb| {
                    let r = rgb[0];
                    let g = rgb[1];
                    let b = rgb[2];

                    0.299 * r + 0.587 * g + 0.114 * b
                }).map(|y| (y * 255.0) as u8)
                .collect::<Vec<u8>>();

            let rgbs_sub = self.next_frame.chunks(3 * ComponentVideoOut::WIDTH * 2)
                .flat_map(|rgbs| {
                    let mut rgbs_sub = vec![];
                    for i in 0..(ComponentVideoOut::WIDTH / 2) {
                        rgbs_sub.push(
                            (rgbs[6*i] + rgbs[6*i + 3] + rgbs[6*i + 3*ComponentVideoOut::WIDTH] + rgbs[6*i + 3*ComponentVideoOut::WIDTH + 3])
                            / 4.0
                        );
                        rgbs_sub.push(
                            (rgbs[6*i + 1] + rgbs[6*i + 4] + rgbs[6*i + 3*ComponentVideoOut::WIDTH + 1] + rgbs[6*i + 3*ComponentVideoOut::WIDTH + 4])
                            / 4.0
                        );
                        rgbs_sub.push(
                            (rgbs[6*i + 2] + rgbs[6*i + 5] + rgbs[6*i + 3*ComponentVideoOut::WIDTH + 2] + rgbs[6*i + 3*ComponentVideoOut::WIDTH + 5])
                            / 4.0
                        );
                    }
                    rgbs_sub
                }).collect::<Vec<f32>>();
            let us = rgbs_sub.chunks(3)
                .map(|rgb| {
                    let r = rgb[0];
                    let g = rgb[1];
                    let b = rgb[2];

                    -0.147 * r - 0.289 * g + 0.436 * b
                }).map(|u| (u * 127.5 + 127.5) as u8)
                .collect::<Vec<u8>>();
            let vs = rgbs_sub.chunks(3)
                .map(|rgb| {
                    let r = rgb[0];
                    let g = rgb[1];
                    let b = rgb[2];

                    0.615 * r - 0.515 * g + 0.100 * b
                }).map(|u| (u * 127.5 + 127.5) as u8)
                .collect::<Vec<u8>>();

            let frame = y4m::Frame::new([&ys, &us, &vs], None);
            self.writer.write_frame(&frame)?;

            self.next_frame = vec![];
        }

        self.next_frame.push(sample.clamp(0.0, 1.0));

        Ok(())
    }
    fn finalize(&mut self) -> Result<(), y4m::Error> {
        Ok(())
    }
}
impl std::fmt::Debug for Y4mWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Y4mWriter {{ filename: \"{}\" }}", self.filename)
    }
}
impl Clone for Y4mWriter {
    fn clone(&self) -> Self {
        Y4mWriter::new(&self.filename)
    }
}

#[derive(Debug)]
enum FileWriterError {
    HoundError(hound::Error),
    Y4mError(y4m::Error),
}

#[derive(Debug, Clone)]
enum FileWriter {
    WavWriter(WavWriter),
    Y4mWriter(Y4mWriter),
}
impl FileWriter {
    fn finalize(&mut self) -> Result<(), FileWriterError> {
        match self {
            FileWriter::WavWriter(writer) => {
                writer.finalize()
                    .map_err(FileWriterError::HoundError)
            },
            FileWriter::Y4mWriter(writer) => {
                writer.finalize()
                    .map_err(FileWriterError::Y4mError)
            },
        }
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
                            TextSection::new(format!("{}\n", self.filename), ts),
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

        if self.filename.ends_with(".wav") {
            self.writer = Some(FileWriter::WavWriter(WavWriter::new(&self.filename)));
        } else if self.filename.ends_with(".y4m") {
            self.writer = Some(FileWriter::Y4mWriter(Y4mWriter::new(&self.filename)))
        } else {
            panic!("Invalid file type for FileEncoder: {}", self.filename);
        }
    }
    fn exit(&mut self) {
        if let Some(writer) = &mut self.writer {
            writer.finalize().unwrap();
        }

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
        match &self.writer {
            Some(FileWriter::WavWriter(_)) => 2,
            Some(FileWriter::Y4mWriter(_)) => 3,
            None => 0,
        }
    }
    fn outputs(&self) -> usize {
        0
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, st: StepType, ins: &[f32]) -> Vec<f32> {
        match &mut self.writer {
            Some(FileWriter::WavWriter(writer)) => {
                if st == StepType::Video {
                    return vec![];
                }

                let left = ins[0];
                let right = if ins[1].is_nan() {
                    left
                } else {
                    ins[1]
                };

                writer.write_sample(left)
                    .unwrap_or_else(|e| panic!("Failed to write sample to WAV file {}: {e}", self.filename));
                writer.write_sample(right)
                    .unwrap_or_else(|e| panic!("Failed to write sample to WAV file {}: {e}", self.filename));
            },
            Some(FileWriter::Y4mWriter(writer)) => {
                writer.write_sample(ins[0])
                    .unwrap_or_else(|e| panic!("Failed to write sample to Y4M file {}: {e}", self.filename));
                writer.write_sample(ins[1])
                    .unwrap_or_else(|e| panic!("Failed to write sample to Y4M file {}: {e}", self.filename));
                writer.write_sample(ins[2])
                    .unwrap_or_else(|e| panic!("Failed to write sample to Y4M file {}: {e}", self.filename));
            },
            None => {},
        }

        vec![]
    }
}
