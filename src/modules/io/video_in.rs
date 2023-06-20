/*!
The `VideoIn` module outputs 3 signals as RGB data from a given video source.

## Video Sources
 * `Screen` - Output video from a region on the screen. Currently hard-coded
   as:
   {
       x: 0,
       y: 0,
       w: [80](ComponentVideoOut::WIDTH),
       h: [60](ComponentVideoOut::HEIGHT)
    }
 * `Camera` - Output video from a region of a webcam. Currently the same as
   above.

## Inputs
None

## Outputs
0. Gamma-corrected red channel in the range [0.0, 1.0]
1. Gamma-corrected green channel in the range [0.0, 1.0]
2. Gamma-corrected blue channel in the range [0.0, 1.0]

##### Note
If the video buffer becomes empty, the outputs will all be [f32::NAN].

## Knobs
None

*/

use std::{sync::{Arc, Mutex}, collections::VecDeque};

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use screenshots::Screen;
use nokhwa::Camera;

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
#[derive(Clone)]
struct CameraSource {
    camera: Arc<Mutex<Camera>>,
    images: Arc<Mutex<Vec<image::ImageBuffer<<nokhwa::pixel_format::RgbFormat as nokhwa::FormatDecoder>::Output, Vec<u8>>>>>,
}
impl std::fmt::Debug for CameraSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CameraSource")
    }
}
#[derive(Deserialize, Debug, Clone)]
enum VideoSource {
    Screen(
        #[serde(skip)]
        Option<ScreenSource>,
    ),
    Camera(
        #[serde(skip)]
        Option<CameraSource>,
    ),
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoIn {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    video_buffer: VecDeque<u8>,

    source: VideoSource,
}
impl VideoIn {
    const GAMMA: f32 = 2.2;

    fn queue_video_in(&mut self) {
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
            VideoSource::Camera(camera) => {
                if camera.is_none() {
                    nokhwa::nokhwa_initialize(|granted| {
                        if !granted {
                            panic!("Failed to acquire camera permissions for video input");
                        }
                    });

                    let mut cam = Camera::new(
                        nokhwa::utils::CameraIndex::Index(0),
                        nokhwa::utils::RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(
                            nokhwa::utils::RequestedFormatType::AbsoluteHighestFrameRate,
                        ),
                    ).expect("Failed to get camera for video input");
                    cam.open_stream().expect("Failed to open camera stream for video input");
                    *camera = Some(CameraSource {
                        camera: Arc::new(Mutex::new(cam)),
                        images: Arc::new(Mutex::new(vec![])),
                    });
                }

                if let Some(camera) = camera {
                    let images = camera.images.clone();
                    let camera = camera.camera.clone();
                    std::thread::spawn(move || {
                        if let Ok(mut camera) = camera.lock() {
                            let image = camera.frame()
                                .expect("Failed to capture camera frame for video input");
                            if let Ok(mut images) = images.lock() {
                                images.push(
                                    image.decode_image::<nokhwa::pixel_format::RgbFormat>()
                                        .expect("Failed to decode image from camera for video input")
                                );
                            }
                        }
                    });
                }
            },
        }
    }
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

        self.queue_video_in();
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
        3
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        // Fetch video input
        if st == StepType::Key {
            self.queue_video_in();
        }

        // Process video input to buffer
        match &mut self.source {
            VideoSource::Screen(screen) => {
                if let Some(screen) = screen {
                    if let Ok(mut images) = screen.images.try_lock() {
                        for image in images.drain(..) {
                            if self.video_buffer.len() < image.bgra().len() {
                                self.video_buffer.extend(image.bgra());
                            }
                        }
                    }
                }
            },
            VideoSource::Camera(camera) => {
                if let Some(camera) = camera {
                    if let Ok(mut images) = camera.images.try_lock() {
                        for image in images.drain(..) {
                            if self.video_buffer.len() < image.len() {
                                self.video_buffer.extend(
                                    image.enumerate_pixels()
                                        .filter_map(|(x, y, p)| {
                                            if x < ComponentVideoOut::WIDTH as u32 && y < ComponentVideoOut::HEIGHT as u32 {
                                                Some([p.0[2], p.0[1], p.0[0], 255])
                                            } else {
                                                None
                                            }
                                        }).flatten()
                                );
                            }
                        }
                    }
                }
            },
        }

        // Pop output from buffer
        if self.video_buffer.is_empty() {
            vec![f32::NAN, f32::NAN, f32::NAN]
        } else {
            let bgra: Vec<u8> = self.video_buffer.drain(0..4).collect();
            let b = f32::from(bgra[0]) / 255.0;
            let g = f32::from(bgra[1]) / 255.0;
            let r = f32::from(bgra[2]) / 255.0;

            let er = r.powf(VideoIn::GAMMA);
            let eg = g.powf(VideoIn::GAMMA);
            let eb = b.powf(VideoIn::GAMMA);

            vec![er, eg, eb]
        }
    }
}
