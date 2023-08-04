/*!
The `FileDecoder` module takes either a WAV file or a Y4M file and outputs it,
looping upon reaching the end.

##### Note
A proper Y4M video file can be produced using `ffmpeg` as follows:

```
$ ffmpeg -i video.mp4 -s 80x60 -f yuv4mpegpipe -filter:v fps=36.75 video.y4m
```

## Inputs
None

## Outputs
 * If given a WAV file:
   0. The left channel of the audio signal
   1. The right channel of the audio signal
 * If given a Y4M file:
   0. The red channel
   1. The green channel
   2. The blue channel

##### Note
If given a WAV file and the right channel is missing, then the left channel
will be copied to both outputs.

##### Note
If the buffer becomes empty, the outputs will all be [f32::NAN].

## Knobs
0. Gain in the range [0.0, inf)

*/

use std::fs::File;

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

pub struct WavReader {
    filename: String,
    reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
    idx: usize,
    buffer: Vec<[f32; 2]>,
}
impl WavReader {
    pub(crate) fn new(filename: &str) -> Self {
        WavReader {
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
    fn rewind(&mut self) {
        // Finish reading before rewinding
        while let Some(s) = self.reader.samples::<i16>().next() {
            self.append_sample(s);
        }

        self.idx = 0;
    }
    fn read_sample(&mut self, should_loop: bool) -> Option<[f32; 2]> {
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
impl std::fmt::Debug for WavReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WavReader {{ filename: \"{}\" }}", self.filename)
    }
}
impl Clone for WavReader {
    fn clone(&self) -> Self {
        WavReader::new(&self.filename)
    }
}

pub struct Y4mReader {
    filename: String,
    reader: y4m::Decoder<std::io::BufReader<std::fs::File>>,
    idx: usize,
    rgb_buffer: Vec<[f32; 3]>,
}
impl Y4mReader {
    pub(crate) fn new(filename: &str) -> Self {
        let file = File::open(filename)
            .unwrap_or_else(|e| panic!("Failed to open Y4M file {}: {e}", filename));
        Y4mReader {
            filename: filename.to_string(),
            reader: y4m::Decoder::new(std::io::BufReader::new(file))
                .unwrap_or_else(|e| panic!("Failed to decode Y4M file {}: {e}", filename)),
            idx: 0,
            rgb_buffer: vec![],
        }
    }
    fn read_all(&mut self) {
        self.rgb_buffer.clear();

        let width = self.reader.get_width();
        let height = self.reader.get_height();
        let js: Vec<usize> = (0..(height/2))
            .flat_map(|row| {
                ((row*width)..(row*width+width))
                    .map(|i| i / 2)
                    .cycle()
                    .take(width * 2)
            }).collect();

        loop {
            match self.reader.read_frame() {
                Ok(frame) => {
                    let ys = frame.get_y_plane();
                    let us = frame.get_u_plane();
                    let vs = frame.get_v_plane();

                    for (i, &y) in ys.iter()
                        .enumerate()
                    {
                        let y = y as f32 / 255.0;

                        let j = js[i];
                        let u = us[j] as f32 / 127.5 - 1.0;
                        let v = vs[j] as f32 / 127.5 - 1.0;

                        // Standard YUV conversion
                        let r = y + 1.140*v;
                        let g = y - 0.395*u - 0.581*v;
                        let b = y + 2.032*u;

                        self.rgb_buffer.push([
                            r.clamp(0.0, 1.0),
                            g.clamp(0.0, 1.0),
                            b.clamp(0.0, 1.0),
                        ]);
                    }
                },
                Err(y4m::Error::EOF) => break,
                Err(e) => panic!("Failed to read Y4M frame from {}: {e}", self.filename),
            }
        }
    }
    fn rewind(&mut self) {
        if self.rgb_buffer.is_empty() {
            self.read_all();
        }

        self.idx = 0;
    }
    fn read_sample(&mut self, should_loop: bool) -> Option<[f32; 3]> {
        if self.rgb_buffer.is_empty() {
            self.read_all();
        }

        self.idx += 1;
        if should_loop {
            self.idx %= self.rgb_buffer.len();
        } else if self.idx > self.rgb_buffer.len() {
            return None;
        }

        if self.idx > 0 {
            Some(self.rgb_buffer[self.idx-1])
        } else {
            Some(self.rgb_buffer[self.rgb_buffer.len() - 1])
        }
    }
}
impl std::fmt::Debug for Y4mReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Y4mReader {{ filename: \"{}\" }}", self.filename)
    }
}
impl Clone for Y4mReader {
    fn clone(&self) -> Self {
        Y4mReader::new(&self.filename)
    }
}

#[derive(Debug, Clone)]
pub enum FileReader {
    WavReader(WavReader),
    Y4mReader(Y4mReader),
}
impl FileReader {
    pub(crate) fn rewind(&mut self) {
        match self {
            FileReader::WavReader(reader) => reader.rewind(),
            FileReader::Y4mReader(reader) => reader.rewind(),
        }
    }
    pub(crate) fn read_sample(&mut self, should_loop: bool) -> Option<Vec<f32>> {
        match self {
            FileReader::WavReader(reader) => reader.read_sample(should_loop).map(|a| a.to_vec()),
            FileReader::Y4mReader(reader) => reader.read_sample(should_loop).map(|a| a.to_vec()),
        }
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
impl FileDecoder {
    fn init_reader(&mut self) {
        if self.filename.ends_with(".wav") {
            self.reader = Some(FileReader::WavReader(WavReader::new(&self.filename)));
        } else if self.filename.ends_with(".y4m") {
            self.reader = Some(FileReader::Y4mReader(Y4mReader::new(&self.filename)));
        } else {
            panic!("Invalid file type for FileDecoder: {}", self.filename);
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

        if self.reader.is_none() {
            self.init_reader()
        }
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
        match self.reader {
            Some(FileReader::WavReader(_)) => 2,
            Some(FileReader::Y4mReader(_)) => 3,
            None => 0,
        }
    }
    fn knobs(&self) -> usize {
        1
    }

    fn step(&mut self, _time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        if self.reader.is_none() {
            self.init_reader()
        }

        match &mut self.reader {
            Some(FileReader::WavReader(reader)) => {
                if st == StepType::Video {
                    return vec![f32::NAN; self.outputs()];
                }

                let sample = reader.read_sample(true).unwrap();
                vec![
                    sample[0] * self.knobs[0],
                    sample[1] * self.knobs[0],
                ]
            },
            Some(FileReader::Y4mReader(reader)) => {
                let sample = reader.read_sample(true).unwrap();
                vec![
                    sample[0] * self.knobs[0],
                    sample[1] * self.knobs[0],
                    sample[2] * self.knobs[0],
                ]
            },
            None => vec![f32::NAN; self.outputs()],
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
