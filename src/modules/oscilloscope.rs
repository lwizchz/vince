/*!
The `Oscilloscope` module takes an input and displays it as a graph of values
over time.

## Inputs
0. The signal to graph

## Outputs
None

## Knobs
None

*/

use std::collections::VecDeque;

use bevy::{prelude::*, ecs::{system::EntityCommands}, sprite::{Mesh2dHandle, MaterialMesh2dBundle}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureDimension}, view::RenderLayers, camera::RenderTarget}, core_pipeline::clear_color::ClearColorConfig};

use serde::Deserialize;

use crate::{StepType, CameraComponent, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
pub struct Oscilloscope {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    mesh: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    max_t: f32,
    #[serde(skip)]
    max_val: f32,
    #[serde(skip)]
    vals: VecDeque<(f32, f32)>,
    #[serde(skip)]
    cycles: usize,
}
impl Oscilloscope {
    const WIDTH: f32 = 150.0;
    const HEIGHT: f32 = 100.0;
    const MAX_LEN: usize = 2048;

    fn gen_points(&mut self) -> Vec<Vec3> {
        match self.vals.front() {
            Some((t0, _)) => {
                let (mut max_t, mut max_val) = self.vals.iter()
                    .fold((0.0f32, 0.0f32), |mut a, (t, v)| {
                        if *t > a.0 {
                            a.0 = *t;
                        }
                        if v.abs() > a.1 {
                            a.1 = v.abs();
                        }
                        a
                    });
                if self.max_t > max_t {
                    max_t = self.max_t;
                }
                if self.max_val > max_val {
                    max_val = self.max_val;
                }
                self.max_t = max_t;
                self.max_val = max_val;
                self.vals.iter()
                    .map(|(t, v)| Vec3 {
                        x: (t - t0) * Self::WIDTH / (max_t - t0),
                        y: v * Self::HEIGHT / max_val,
                        z: 0.0,
                    }).collect::<Vec<Vec3>>()
            },
            None => vec![],
        }
    }
}
#[typetag::deserialize]
impl Module for Oscilloscope {
    fn init(&mut self, id: usize, mut ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
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
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(size);
        let image_handle = images.add(image);

        let layer = RenderLayers::layer(((id+1) % 255) as u8);
        let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.gen_points());
        self.mesh = Some(ec.commands().spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(mesh)),
                material: materials.add(ColorMaterial::from(Color::GREEN)),
                transform: Transform::from_xyz(-Self::WIDTH/2.0, 0.0, 0.0)
                    .with_scale(Vec3 {
                        x: 1.0,
                        y: 0.5,
                        z: 1.0,
                    }),
                ..default()
            },
            ModuleMeshComponent,
            layer,
        )).id());
        ec.commands().spawn((
            Camera2dBundle {
                camera_2d: Camera2d {
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                },
                camera: Camera {
                    order: -1,
                    target: RenderTarget::Image(image_handle.clone()),
                    ..default()
                },
                ..default()
            },
            UiCameraConfig {
                show_ui: false,
            },
            CameraComponent,
            layer,
        ));

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
                    None => format!("M{id} Oscilloscope\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("LEVEL".to_string(), ts),
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

        self.vals = VecDeque::with_capacity(512);
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

    fn step(&mut self, time: f32, ft: StepType, ins: &[f32]) -> Vec<f32> {
        if ft == StepType::Video {
            return vec![];
        }

        let val = ins[0];

        if !self.vals.is_empty() {
            if val.signum() != self.vals.iter().last().unwrap().1.signum() {
                self.cycles += 1;
            }
        }
        if self.cycles >= 14 {
            self.vals.pop_front();
            self.cycles -= 1;
        } else if self.vals.len() > Self::MAX_LEN {
            self.vals.pop_front();
        }
        self.vals.push_back((time, val));

        vec![]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                let val = self.vals.back().unwrap_or(&(0.0, 0.0));
                text.sections[1].value = format!("{}", val.1);
            }
        }

        if let Some(component) = self.mesh {
            if let Ok(h_mesh) = q_mesh.get_mut(component) {
                if let Some(mesh) = meshes.get_mut(&h_mesh.0) {
                    if let Some(attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                        *attr = self.gen_points().into();
                    }
                }
            }
        }
    }
}
