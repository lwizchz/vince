use bevy::{prelude::*, ecs::{system::EntityCommands}, sprite::{Mesh2dHandle, MaterialMesh2dBundle}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureDimension}, view::RenderLayers, camera::RenderTarget}, core_pipeline::clear_color::ClearColorConfig};

use serde::Deserialize;

use crate::{CameraComponent, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
pub struct Oscilloscope {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    mesh: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    #[serde(default)]
    max_val: Option<f32>,
    #[serde(default)]
    vals: Vec<(f32, f32)>,
}
impl Oscilloscope {
    const WIDTH: f32 = 150.0;
    const HEIGHT: f32 = 100.0;
    const MAX_LEN: usize = 64;

    fn gen_points(&mut self) -> Vec<Vec3> {
        if self.vals.is_empty() {
            return vec![];
        }

        let t0 = self.vals.first().unwrap().0;
        let (max_t, mut max_val) = self.vals.iter()
            .fold((0.0f32, 0.0f32), |mut a, (t, v)| {
                if *t > a.0 {
                    a.0 = *t;
                }
                if v.abs() > a.1 {
                    a.1 = v.abs();
                }
                a
            });
        if let Some(mv) = self.max_val {
            if mv > max_val {
                max_val = mv;
            }
        }
        self.max_val = Some(max_val);
        self.vals.iter()
            .map(|(t, v)| Vec3 {
                x: (t - t0) * Self::WIDTH / (max_t - t0),
                y: v * Self::HEIGHT / max_val,
                z: 0.0,
            }).collect::<Vec<Vec3>>()
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

        let layer = RenderLayers::layer((id+1) as u8);
        self.mesh = Some(ec.commands().spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Mesh::from(LineStrip {
                    points: self.gen_points(),
                }))),
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
                    clear_color: ClearColorConfig::Custom(Color::NONE),
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
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            top: Val::Px(5.0),
                            left: Val::Px(5.0),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(format!("M{id} Oscilloscope\n"), ts.clone()),
                            TextSection::new("LEVEL".to_string(), ts),
                        ]).with_style(Style {
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                top: Val::Px(5.0),
                                left: Val::Px(5.0),
                                ..default()
                            },
                            ..default()
                        }),
                        ModuleTextComponent,
                    )).id()
                );

                self.children.push(
                    parent.spawn((
                        ImageBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                position: UiRect {
                                    top: Val::Px(Self::HEIGHT/2.0 + 5.0),
                                    left: Val::Px(0.0),
                                    ..default()
                                },
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

        self.vals = vec![];
        self.vals.reserve(Self::MAX_LEN);
        self.vals.push((0.0, 0.0));
    }
    fn is_init(&self) -> bool {
        self.id.is_some()
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

    fn get_knobs(&self) -> Vec<f32> {
        vec![]
    }
    fn set_knob(&mut self, _i: usize, _val: f32) {}

    fn step(&mut self, time: f32, ins: &[f32]) -> Vec<f32> {
        let val = ins[0];

        if self.vals.len() == Self::MAX_LEN {
            self.vals.remove(0);
        }
        self.vals.push((time, val));

        vec![]
    }
    fn render(&mut self, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                let val = self.vals.last().unwrap();
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
#[derive(Debug, Clone)]
pub struct LineStrip {
    pub points: Vec<Vec3>,
}
impl From<LineStrip> for Mesh {
    fn from(line: LineStrip) -> Self {
        let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, line.points);
        mesh
    }
}
