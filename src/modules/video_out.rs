use std::collections::VecDeque;

use bevy::{prelude::*, ecs::{system::EntityCommands}, sprite::Mesh2dHandle, render::{render_resource::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureDimension}}};

use serde::Deserialize;

use crate::{MainCameraComponent, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct VideoOut {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    #[serde(default)]
    scan: usize,
    #[serde(default)]
    luma: VecDeque<(f32, f32)>,
    #[serde(default)]
    chroma: VecDeque<(f32, f32)>,
}
impl VideoOut {
    pub const WIDTH: f32 = 640.0;
    pub const HEIGHT: f32 = 480.0;
    const MAX_LEN: usize = 2048;
}
#[typetag::deserialize]
impl Module for VideoOut {
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
                                size: Size::new(Val::Px(Self::WIDTH), Val::Px(Self::HEIGHT)),
                                ..default()
                            },
                            image: UiImage::new(image_handle),
                            ..default()
                        },
                        ModuleImageComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });
    }
    fn is_large(&self) -> bool {
        true
    }
    fn get_pos(&self, q_child: &Query<&Parent, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>, q_camera: &Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>) -> Vec3 {
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

    fn step(&mut self, time: f32, ins: &[f32]) -> Vec<f32> {
        let y = ins[0];
        if self.luma.len() > Self::MAX_LEN {
            self.luma.remove(0);
        }
        self.luma.push_back((time, y));

        let c = ins[1];
        if self.chroma.len() > Self::MAX_LEN {
            self.chroma.remove(0);
        }
        self.chroma.push_back((time, c));

        vec![]
    }
    fn render(&mut self, images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(1) {
            if let Ok(h_image) = q_image.get_mut(*component) {
                if let Some(image) = images.get_mut(&h_image.texture) {
                    for (luma, chroma) in self.luma.drain(0..).zip(self.chroma.drain(0..)) {
                        let y = luma.1;
                        let c = chroma.1;
                        let i = c;
                        let q = c;

                        // FIXME gamma correction
                        image.data[self.scan] = ((y + 0.9469*i + 0.6236*q)*255.0) as u8;
                        image.data[self.scan+1] = ((y - 0.2748*i - 0.6357*q)*255.0) as u8;
                        image.data[self.scan+2] = ((y - 1.1*i + 1.7*q)*255.0) as u8;
                        image.data[self.scan+3] = 255;

                        self.scan = (self.scan+4) % image.data.len();
                    }
                }
            }
        }
    }
}