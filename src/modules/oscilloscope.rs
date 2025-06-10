/*!
The `Oscilloscope` module takes up to 4 inputs and displays them as graphs of
values over time.

## Inputs
0. The first signal to graph
1. The second signal to graph
2. The third signal to graph
3. The fourth signal to graph

## Outputs
None

## Knobs
None

*/

use std::collections::VecDeque;

use bevy::{color::palettes, ecs::system::EntityCommands, prelude::*, render::{camera::{ClearColorConfig, RenderTarget}, render_asset::RenderAssetUsages, render_resource::{Extent3d, PrimitiveTopology, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages}, view::RenderLayers}};

use serde::Deserialize;

use crate::{StepType, CameraComponent, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent, ModuleImageWindowComponent}};

#[derive(Default, Deserialize, Debug, Clone)]
pub struct Oscilloscope {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    mesh: Option<[Entity; Oscilloscope::MAX_GRAPHS]>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    max_t: f64,
    #[serde(skip)]
    max_val: [f32; Oscilloscope::MAX_GRAPHS],
    #[serde(skip)]
    vals: [VecDeque<(f64, f32)>; Oscilloscope::MAX_GRAPHS],
    #[serde(skip)]
    cycles: [usize; Oscilloscope::MAX_GRAPHS],

    #[serde(default)]
    is_own_window: bool,
}
impl Oscilloscope {
    const WIDTH: usize = 150;
    const HEIGHT: usize = 100;
    const MAX_LEN: usize = 2048;
    const MAX_GRAPHS: usize = 4;

    fn gen_points(&mut self) -> [Vec<Vec3>; Oscilloscope::MAX_GRAPHS] {
        let mut points: [Vec<Vec3>; Oscilloscope::MAX_GRAPHS] = vec![vec![]; Oscilloscope::MAX_GRAPHS]
            .try_into()
            .unwrap();
        for (i, point) in points.iter_mut().enumerate() {
            if let Some((t0, _)) = self.vals[i].front() {
                if self.max_val[i] > 0.0 {
                    self.max_val[i] /= 1.05;
                }

                let (mut max_t, mut max_val) = self.vals[i].iter()
                    .fold((0.0f64, 0.0f32), |mut a, (t, v)| {
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
                if self.max_val[i] > max_val {
                    max_val = self.max_val[i];
                }
                self.max_t = max_t;
                self.max_val[i] = max_val;

                *point = self.vals[i].iter()
                    .map(|(t, v)| Vec3 {
                        x: ((t - t0) * f64::from(Self::WIDTH as u16) / (max_t - t0)) as f32,
                        y: v * f32::from(Self::HEIGHT as u16) / max_val,
                        z: 0.0,
                    }).collect::<Vec<Vec3>>();
            }
        }
        points
    }
}
#[typetag::deserialize]
impl Module for Oscilloscope {
    fn init(&mut self, id: usize, mut ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor)) {
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

        let layer = RenderLayers::layer((id+1) % 255);
        let mut mesh: [Mesh; Oscilloscope::MAX_GRAPHS] = vec![Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::all()); Oscilloscope::MAX_GRAPHS]
            .try_into()
            .unwrap();
        for (i, gen_points) in self.gen_points().into_iter().enumerate() {
            mesh[i].insert_attribute(Mesh::ATTRIBUTE_POSITION, gen_points);
        }
        let colors = [
            palettes::css::LIME,
            palettes::css::RED,
            palettes::css::YELLOW,
            palettes::css::BLUE,
        ];
        self.mesh = Some(
            mesh.into_iter().enumerate()
                .map(|(i, mesh)| {
                    ec.commands().spawn((
                        Mesh2d(meshes.add(mesh)),
                        MeshMaterial2d(materials.add(Color::Srgba(colors[i]))),
                        Transform::from_xyz(-f32::from(Self::WIDTH as u16)/2.0, 0.0, 0.0)
                            .with_scale(Vec3 {
                                x: 1.0,
                                y: 0.5,
                                z: 1.0,
                            }),
                        ModuleMeshComponent,
                        layer.clone(),
                    )).id()
                }).collect::<Vec<Entity>>()
                .try_into()
                .unwrap()
        );
        ec.commands().spawn((
            Camera2d,
            Camera {
                order: -1,
                target: RenderTarget::Image(image_handle.clone().into()),
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            CameraComponent,
            layer,
        ));

        ec.with_children(|parent| {
            let mut component = parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    flex_direction: FlexDirection::Column,
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
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["Average\n", "Max\n"] {
                            p.spawn((
                                TextSpan::new(t),
                                tfc.0.clone(),
                                tfc.1.clone(),
                            ));
                        }
                    }).id()
                );

                self.children.push(
                    parent.spawn((
                        ImageNode::new(image_handle.clone()),
                        Node {
                            position_type: PositionType::Relative,
                            top: Val::Px(10.0),
                            width: Val::Px(f32::from(Self::WIDTH as u16)),
                            height: Val::Px(f32::from(Self::HEIGHT as u16)),
                            ..default()
                        },
                        ModuleImageComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        if self.is_own_window() {
            ec.commands().spawn((
                Sprite {
                    image: image_handle,
                    custom_size: Some(Vec2::new(150.0, 100.0)),
                    ..default()
                },
                Transform::from_xyz(640.0*id as f32, 1080.0*2.0, 0.0),
                ModuleImageWindowComponent,
            ));
        }

        self.vals = vec![VecDeque::with_capacity(512); Oscilloscope::MAX_GRAPHS]
            .try_into()
            .unwrap();
    }
    fn exit(&mut self) {
        self.id = None;
        self.component = None;
        self.mesh = None;
        self.children = vec![];
    }
    fn is_own_window(&self) -> bool {
        self.is_own_window
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
        Oscilloscope::MAX_GRAPHS
    }
    fn outputs(&self) -> usize {
        0
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, time: f64, st: StepType, ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![];
        }

        for (i, val) in ins.iter().enumerate() {
            if !self.vals[i].is_empty() && val.signum() != self.vals[i].iter().last().unwrap().1.signum() {
                self.cycles[i] += 1;
            }
            if self.cycles[i] >= 14 {
                self.vals[i].pop_front();
                self.cycles[i] -= 1;
            } else if self.vals[i].len() > Self::MAX_LEN {
                self.vals[i].pop_front();
            }
            self.vals[i].push_back((time, *val));
        }

        vec![]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let avg = self.vals.iter().map(|vals| {
                vals.back().unwrap_or(&(0.0, f32::NAN)).1
            }).fold(0.0f32, |a, v| {
                if !v.is_nan() {
                    a + v
                } else {
                    a
                }
            }) / Oscilloscope::MAX_GRAPHS as f32;
            let max = self.max_val.iter()
                .fold(0.0f32, |a, m| {
                    if !m.is_nan() && *m > a {
                        *m
                    } else {
                        a
                    }
                });

            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    format!("Average: {:+}\n", avg),
                    format!("Max: {}\n", max),
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }

        for (i, gen_points) in self.gen_points().into_iter().enumerate() {
            if let Some(component) = self.mesh {
                if let Ok(h_mesh) = q_mesh.get_mut(component[i]) {
                    if let Some(mesh) = meshes.get_mut(&h_mesh.0) {
                        if let Some(attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                            *attr = gen_points.into();
                        }
                    }
                }
            }
        }
    }
}
