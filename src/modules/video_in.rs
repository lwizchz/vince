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
        match ft {
            StepType::Key => match &mut self.source {
                VideoSource::Screen(screen) => {
                    if screen.is_none() {
                        *screen = Some(ScreenSource {
                            screen: Screen::all().expect("Failed to get screens for video input")
                                .first().expect("Failed to get screens for video input")
                                .clone(),
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
            },
            _ => {},
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
        if !self.video_buffer.is_empty() {
            let b = self.video_buffer.pop_front().unwrap() as f32 / 255.0;
            let g = self.video_buffer.pop_front().unwrap() as f32 / 255.0;
            let r = self.video_buffer.pop_front().unwrap() as f32 / 255.0;
            let _a = self.video_buffer.pop_front().unwrap() as f32 / 255.0;

            let er = r.powf(VideoIn::GAMMA);
            let eg = g.powf(VideoIn::GAMMA);
            let eb = b.powf(VideoIn::GAMMA);

            // let ey = 0.30 * er + 0.59 * eg + 0.11 * eb;
            // let ei = -0.27 * (eb - ey) + 0.74 * (er - ey);
            // let eq = 0.41 * (eb - ey) + 0.48 * (er - ey);
            // let ec = ei * eq; // FIXME properly modulate chroma

            // vec![ey, ec]

            vec![er, eg, eb]
        } else {
            vec![0.0, 0.0, 0.0]
        }
    }
}
