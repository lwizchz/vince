/*!
The `ComponentVideoOut` module takes 3 inputs and displays them as RGB data on
a [80](ComponentVideoOut::WIDTH)x[60](ComponentVideoOut::HEIGHT) screen which
is upscaled to 640x480.

## Inputs
0. Red channel
1. Green channel
2. Blue channel

## Outputs
None

## Knobs
None

*/

use std::collections::VecDeque;

use bevy::{prelude::*, ecs::{system::EntityCommands}, sprite::Mesh2dHandle, render::{render_resource::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureDimension}}};

use serde::Deserialize;

use crate::{StepType, MainCameraComponent, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct ComponentVideoOut {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    scan: usize,
    #[serde(skip)]
    rgb: VecDeque<(f64, [f32; 3])>,

    #[serde(default)]
    is_own_window: bool,
}
impl ComponentVideoOut {
    pub const WIDTH: usize = 80;
    pub const HEIGHT: usize = 60;
    const MAX_LEN: usize = 4096;
}
#[typetag::deserialize]
impl Module for ComponentVideoOut {
    fn init(&mut self, id: usize, mut ec: EntityCommands, images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);

        let size = Extent3d {
            width: Self::WIDTH as u32,
            height: Self::HEIGHT as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(size);
        image.data = [0, 0, 0, 255].repeat(image.data.len() / 4);
        let image_handle = images.add(image);

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
                    None => format!("M{id} Video Out\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );

                self.children.push(
                    parent.spawn((
                        ImageBundle {
                            style: Style {
                                position_type: PositionType::Relative,
                                position: UiRect::top(Val::Px(10.0)),
                                size: Size::new(
                                    Val::Px(f32::from(640u16)),
                                    Val::Px(f32::from(480u16)),
                                ),
                                ..default()
                            },
                            image: UiImage::new(image_handle.clone()),
                            ..default()
                        },
                        ModuleImageComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        if self.is_own_window() {
            ec.commands().spawn(
                SpriteBundle {
                    texture: image_handle,
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(640.0, 480.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(640.0*id as f32, 1080.0*2.0, 0.0),
                    ..default()
                }
            );
        }
    }
    fn is_large(&self) -> bool {
        true
    }
    fn is_own_window(&self) -> bool {
        self.is_own_window
    }
    fn get_world_pos(&self, q_child: &Query<&Parent, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>, q_camera: &Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>) -> Vec3 {
        if let Some(component) = self.component() {
            if let Ok(parent) = q_child.get(component) {
                if let Ok(pos_screen) = q_transform.get(parent.get()) {
                    if let Ok(camera) = q_camera.get_single() {
                        if let Some(pos_world) = camera.0.viewport_to_world(camera.1, pos_screen.translation().truncate()) {
                            return Vec3::from((pos_world.origin.truncate(), 0.0))
                                * Vec3::new(1.0, -1.0, 1.0)
                                + Vec3::new(0.0, -250.0, 0.0);
                        }
                    }
                }
            }
        }
        Vec3::ZERO
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
        3
    }
    fn outputs(&self) -> usize {
        0
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let mut r = ins[0];
        let mut g = ins[1];
        let mut b = ins[2];

        if r.is_nan() && g.is_nan() && b.is_nan() {
            return vec![];
        }
        if r.is_nan() {
            r = 0.0;
        }
        if g.is_nan() {
            g = 0.0;
        }
        if b.is_nan() {
            b = 0.0;
        }

        r = r.clamp(0.0, 1.0);
        g = g.clamp(0.0, 1.0);
        b = b.clamp(0.0, 1.0);

        if self.rgb.len() > Self::MAX_LEN {
            self.rgb.remove(0);
        }
        self.rgb.push_back((time, [r, g, b]));

        vec![]
    }
    fn render(&mut self, images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(1) {
            if let Ok(h_image) = q_image.get_mut(*component) {
                if let Some(image) = images.get_mut(&h_image.texture) {
                    for rgb in self.rgb.drain(..) {
                        let r = (rgb.1[0] * 255.0) as u8;
                        let g = (rgb.1[1] * 255.0) as u8;
                        let b = (rgb.1[2] * 255.0) as u8;

                        image.data[self.scan] = r;
                        image.data[self.scan+1] = g;
                        image.data[self.scan+2] = b;
                        image.data[self.scan+3] = 255;

                        self.scan = (self.scan+4) % image.data.len();
                    }
                }
            }
        }
    }
}
