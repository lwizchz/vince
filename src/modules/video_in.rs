/*!
The `VideoIn` module outputs 3 signals as RGB data from a given video source.

## Video Sources
 * `Screen` - Output video from a region on the screen (currently hard-coded as
   {
       x: 0,
       y: 0,
       w: [640](ComponentVideoOut::WIDTH),
       h: [480](ComponentVideoOut::HEIGHT)
    }
 * <strike>`Camera` - Output video from a webcam</strike> Not yet supported

## Inputs
None

## Outputs
0. Gamma-corrected red channel in the range [0.0, 1.0]
1. Gamma-corrected green channel in the range [0.0, 1.0]
2. Gamma-corrected blue channel in the range [0.0, 1.0]

##### Note
If the video buffer becomes empty, the outputs will all be -1.0

## Knobs
None

*/

use std::{sync::{Arc, Mutex}, collections::VecDeque};

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;
use screenshots::Screen;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, component_video_out::ComponentVideoOut}};

#[derive(Clone)]
struct ScreenSource {
    screen: Screen,
    images: Arc<Mutex<Vec<screenshots::Image>>>,
}
impl std::fmt::Debug for ScreenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScreenSource")
    }
}
#[derive(Deserialize, Debug, Clone)]
enum VideoSource {
    Screen(
        #[serde(skip)]
        Option<ScreenSource>,
    ),
    // Camera(
    //     #[serde(skip)]
    //     Option<CameraSource>,
    // ),
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoIn {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    #[serde(default)]
    video_buffer: VecDeque<u8>,

    source: VideoSource,
}
impl VideoIn {
    const GAMMA: f32 = 2.2;
}
#[typetag::deserialize]
impl Module for VideoIn {
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
                    None => format!("M{id} Video In\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts),
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
        3
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f32, ft: StepType, _ins: &[f32]) -> Vec<f32> {
        // Fetch video input
        if ft == StepType::Key {
            match &mut self.source {
                VideoSource::Screen(screen) => {
                    if screen.is_none() {
                        *screen = Some(ScreenSource {
                            screen: *Screen::all().expect("Failed to get screens for video input")
                                .first().expect("Failed to get screens for video input"),
                            images: Arc::new(Mutex::new(vec![])),
                        });
                    }
                    if let Some(screen) = screen {
                        let screen = screen.clone();
                        let images = screen.images.clone();
                        std::thread::spawn(move || {
                            let image = screen.screen
                                .capture_area(0, 0, ComponentVideoOut::WIDTH as u32, ComponentVideoOut::HEIGHT as u32)
                                .expect("Failed to capture screen for video input");
                            if let Ok(mut images) = images.lock() {
                                images.push(image);
                            }
                        });
                    }
                },
            }
        }

        // Process video input to buffer
        match &mut self.source {
            VideoSource::Screen(screen) => {
                if let Some(screen) = screen {
                    if let Ok(mut images) = screen.images.try_lock() {
                        // FIXME put all images in the video buffer
                        for image in images.drain(0..) {
                            if self.video_buffer.is_empty() {
                                self.video_buffer.extend(image.bgra());
                            }
                        }
                    }
                }
            },
        }

        // Pop output from buffer
        if self.video_buffer.is_empty() {
            vec![-1.0, -1.0, -1.0]
        } else {
            let b = f32::from(self.video_buffer.pop_front().unwrap()) / 255.0;
            let g = f32::from(self.video_buffer.pop_front().unwrap()) / 255.0;
            let r = f32::from(self.video_buffer.pop_front().unwrap()) / 255.0;
            let _a = f32::from(self.video_buffer.pop_front().unwrap()) / 255.0;

            let er = r.powf(VideoIn::GAMMA);
            let eg = g.powf(VideoIn::GAMMA);
            let eb = b.powf(VideoIn::GAMMA);

            vec![er, eg, eb]
        }
    }
}
