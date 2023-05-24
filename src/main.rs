use std::time::Duration;
use std::{f32::consts::PI, env};
use std::collections::HashMap;

use bevy::app::AppExit;
use bevy::asset::LoadState;
use bevy::reflect::TypeUuid;
use bevy::window::PrimaryWindow;
use bevy::{prelude::*, ecs::{system::EntityCommands}, window::WindowResolution, sprite::{Mesh2dHandle, MaterialMesh2dBundle}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureDimension}, view::RenderLayers, camera::RenderTarget}, core_pipeline::clear_color::ClearColorConfig};
use bevy_common_assets::toml::TomlAssetPlugin;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use serde::Deserialize;

const FRAME_RATE: u32 = 60;
const DOWNSAMPLE: u64 = 2;
static mut SAMPLE_RATE: u32 = 44100;
static mut AUDIO_STREAM: Option<cpal::Stream> = None;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes: true,
            ..default()
        }).set(WindowPlugin {
            primary_window: Some(Window {
                title: "Vince Video Synth".to_string(),
                resolution: WindowResolution::new(1920.0, 1080.0),
                resizable: false,
                ..default()
            }),
            ..default()
        })).add_plugin(TomlAssetPlugin::<Rack>::new(&["toml"]))
        .add_plugin(bevy_framepace::FramepacePlugin)
        .add_state::<AppState>()
        .add_startup_system(load_rack)
        .add_system(setup.run_if(in_state(AppState::Loading)))
        .add_system(rack_reloader.run_if(in_state(AppState::Ready)))
        .add_system(rack_stepper.in_schedule(CoreSchedule::FixedUpdate).run_if(in_state(AppState::Ready)))
        .add_system(rack_render.run_if(in_state(AppState::Ready)))
        .add_system(bevy::window::close_on_esc)
        .insert_resource(FixedTime::new_from_secs(1.0 / FRAME_RATE as f32))
        .run();
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    Loading,
    Ready,
}

#[derive(Component)]
struct CameraComponent;

fn load_rack(mut commands: Commands, asset_server: Res<AssetServer>, mut settings_fp: ResMut<bevy_framepace::FramepaceSettings>, mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    settings_fp.limiter = bevy_framepace::Limiter::from_framerate(FRAME_RATE as f64);

    // Load rack from config
    let rack_path = if let Some(rack_path) = env::args().nth(1) {
        rack_path
    } else {
        "racks/rack0.toml".to_string()
    };
    let h_rack = RackHandle(asset_server.load(rack_path.clone()));
    commands.insert_resource(h_rack.clone());

    if let Ok(mut window) = q_window.get_single_mut() {
        window.title = format!("Vince Video Synth - {rack_path}");
    }
}
fn setup(mut commands: Commands, h_rack: ResMut<RackHandle>, mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>, mut state: ResMut<NextState<AppState>>, mut exit: EventWriter<AppExit>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        // Main camera
        commands.spawn((
            Camera2dBundle {
                transform: Transform::from_xyz(1920.0/2.0, -1080.0/2.0, 0.0)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            CameraComponent,
        ));

        // Module rects
        let ts = TextStyle {
            font: asset_server.load("fonts/liberation_mono.ttf"),
            font_size: 16.0,
            color: Color::WHITE,
        };
        for m in &mut rack.modules {
            let id = m.0.parse::<usize>().unwrap();
            m.1.init(
                id,
                commands.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                top: Val::Px(5.0),
                                left: Val::Px(5.0 + 150.0 * id as f32),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    },
                    TopModuleComponent,
                )),
                &mut images,
                &mut meshes,
                &mut materials,
                ts.clone(),
            );
        }

        // Setup audio
        rack.init_audio();

        state.set(AppState::Ready);
    } else if asset_server.get_load_state(&h_rack.0) == LoadState::Failed {
        let rack_path = if let Some(rack_path) = env::args().nth(1) {
            rack_path
        } else {
            "racks/rack0.toml".to_string()
        };
        error!("Invalid file path: {}", rack_path);
        exit.send(AppExit);
    }
}

#[typetag::deserialize(tag = "type")]
pub trait Module: std::fmt::Debug + ModuleClone + Send + Sync {
    fn init(&mut self, id: usize, ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle);
    fn is_init(&self) -> bool;

    fn inputs(&self) -> usize;
    fn outputs(&self) -> usize;
    fn knobs(&self) -> usize;

    fn get_knobs(&self) -> Vec<f32>;
    fn set_knob(&mut self, i: usize, val: f32);

    fn step(&mut self, time: f32, ins: &[f32]) -> Vec<f32>;
    fn render(&mut self, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>);
}
pub trait ModuleClone {
    fn clone_box(&self) -> Box<dyn Module>;
}
impl<T> ModuleClone for T
where
    T: 'static + Module + Clone,
{
    fn clone_box(&self) -> Box<dyn Module> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn Module> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
#[derive(Component, Debug, Clone)]
pub struct TopModuleComponent;
#[derive(Component, Debug, Clone)]
pub struct ModuleComponent;
#[derive(Component, Debug, Clone)]
pub struct ModuleTextComponent;
#[derive(Component, Debug, Clone)]
pub struct ModuleMeshComponent;
#[derive(Component, Debug, Clone)]
pub struct ModuleImageComponent;

#[derive(Deserialize, Debug, Clone)]
pub struct Mixer {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    component: Option<Entity>,

    knobs: [f32; 2],
}
#[typetag::deserialize]
impl Module for Mixer {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn(ModuleComponent);
            component.insert(ModuleTextComponent)
                .insert(
                    TextBundle::from_sections([
                        TextSection::new(format!("M{id} Mixer\n"), ts.clone()),
                    ]).with_style(Style {
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            top: Val::Px(5.0),
                            left: Val::Px(5.0),
                            ..default()
                        },
                        ..default()
                    }),
                );
            self.component = Some(component.id());
        });
    }
    fn is_init(&self) -> bool {
        self.id.is_some()
    }

    fn inputs(&self) -> usize {
        2
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        2
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
    }

    fn step(&mut self, _time: f32, ins: &[f32]) -> Vec<f32> {
        vec![
            ins.iter()
                .zip(self.knobs.iter())
                .fold(0.0, |a, (i, k)| a + i * k)
        ]
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {

    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Multiplier {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,
}
#[typetag::deserialize]
impl Module for Multiplier {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);
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
                            TextSection::new(format!("M{id} Multiplier\n"), ts.clone()),
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
            });
            self.component = Some(component.id());
        });
    }
    fn is_init(&self) -> bool {
        self.id.is_some()
    }

    fn inputs(&self) -> usize {
        2
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        0
    }

    fn get_knobs(&self) -> Vec<f32> {
        vec![]
    }
    fn set_knob(&mut self, _i: usize, _val: f32) {}

    fn step(&mut self, _time: f32, ins: &[f32]) -> Vec<f32> {
        vec![
            ins.iter()
                .product()
        ]
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {

    }
}

#[derive(Default, Deserialize, Debug, Clone)]
enum OscillatorFunc {
    #[default]
    Sine,
    Triangle,
    Square,
}
#[derive(Default, Deserialize, Debug, Clone)]
pub struct Oscillator {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    func: OscillatorFunc,
    knobs: [f32; 4],
}
#[typetag::deserialize]
impl Module for Oscillator {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);
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
                            TextSection::new(format!("M{id} Oscillator\n"), ts.clone()),
                            TextSection::new(format!("K0\n"), ts.clone()),
                            TextSection::new(format!("K1\n"), ts.clone()),
                            TextSection::new(format!("K2\n"), ts.clone()),
                            TextSection::new(format!("K3\n"), ts),
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
            });
            self.component = Some(component.id());
        });
    }
    fn is_init(&self) -> bool {
        self.id.is_some()
    }

    fn inputs(&self) -> usize {
        0
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        2
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
    }

    fn step(&mut self, time: f32, _ins: &[f32]) -> Vec<f32> {
        let t = time;
        let shift = self.knobs[0];
        let speed = self.knobs[1];
        let depth = self.knobs[2];
        let phase = self.knobs[3];

        let val = match self.func {
            OscillatorFunc::Sine => (speed * t * 2.0*PI - phase).sin() * depth + shift,
            OscillatorFunc::Triangle => 2.0 * depth / PI * ((speed * t * 2.0*PI - phase).sin()).asin() + shift,
            OscillatorFunc::Square => if ((speed * t * 2.0*PI - phase).sin() * depth) > 0.0 { depth+shift } else { -depth+shift },
        };

        vec![val]
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Shift {}\n", self.knobs[0]);
                text.sections[2].value = format!("K1 Speed {}\n", self.knobs[1]);
                text.sections[3].value = format!("K2 Depth {}\n", self.knobs[2]);
                text.sections[4].value = format!("K3 Phase {}\n", self.knobs[3]);
            }
        }
    }
}

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
                            TextSection::new(format!("LEVEL"), ts),
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

#[derive(Deserialize, Debug, Clone)]
pub struct Patch(String, String);

pub struct AudioOut {
    _host: cpal::Host,
    _device: cpal::Device,
    config: cpal::StreamConfig,
    mixer_handle: oddio::Handle<oddio::Mixer<[f32; 2]>>,
    buffer: Vec<f32>,
}
impl std::fmt::Debug for AudioOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AudioOut")
    }
}

#[derive(Deserialize, TypeUuid, Debug)]
#[uuid = "23f4f379-ed3e-4e41-9093-58b4e73ea9a9"]
pub struct Rack {
    #[serde(skip)]
    audio_out: Option<AudioOut>,

    pub modules: HashMap<String, Box<dyn Module>>,
    pub patches: Vec<Patch>,
}
impl Rack {
    pub fn new(mut mods: Vec<Box<dyn Module>>, pats: &[(&str, &str)]) -> Self {
        Self {
            audio_out: None,

            modules: mods.drain(0..)
                .enumerate()
                .map(|(i, m)| (format!("{i}"), m.clone()))
                .collect(),
            patches: pats.iter()
                .map(|(p0, p1)| Patch(p0.to_string(), p1.to_string()))
                .collect(),
        }
    }

    fn init_audio(&mut self) {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no audio output device available");
        let sample_rate = device.default_output_config().unwrap().sample_rate();
        unsafe {
            if sample_rate.0 != SAMPLE_RATE {
                SAMPLE_RATE = sample_rate.0;
            }
        }

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (mixer_handle, mixer) = oddio::split(oddio::Mixer::new());

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = oddio::frame_stereo(data);
                oddio::run(&mixer, sample_rate.0, frames);
            },
            move |err| {
                error!("{err}");
            },
            None,
        ).unwrap();
        stream.play().unwrap();
        unsafe {
            AUDIO_STREAM = Some(stream);
        }

        self.audio_out = Some(AudioOut {
            _host: host,
            _device: device,
            config: config,
            mixer_handle,
            buffer: vec![],
        });
    }

    pub fn step(&mut self, time: f32) {
        if self.audio_out.is_none() {
            self.init_audio();
        }

        let mut stepped: Vec<String> = Vec::with_capacity(self.modules.len());
        let mut outs: HashMap<String, f32> = HashMap::with_capacity(self.modules.iter().fold(0, |a, m| a + m.1.outputs()));

        // Step all modules which take no inputs
        for (idx, m) in self.modules.iter_mut()
            .filter(|(idx, m)|
                m.inputs() == 0
                || self.patches.iter()
                    .find(|p| p.1.starts_with(&format!("{idx}M")))
                    .is_none()
            )
        {
            let mouts = m.step(time, &vec![0.0; m.inputs()]);
            stepped.push(idx.to_owned());
            for (i, mo) in mouts.iter().enumerate() {
                outs.insert(format!("{idx}M{i}O"), *mo);
            }
        }

        // Step all other modules
        while stepped.len() < self.modules.len() {
            for (idx, m) in &mut self.modules {
                if !stepped.contains(idx) {
                    let inpatches: Vec<&Patch> = self.patches.iter()
                        .filter(|p| p.1.starts_with(&format!("{idx}M")))
                        .collect();
                    if inpatches.iter()
                        .all(|p| outs.iter()
                            .find(|o| o.0 == &p.0)
                            .is_some()
                        )
                    {
                        let mut mins = vec![0.0; m.inputs()];

                        for p in inpatches {
                            match outs.iter()
                                .find(|o| o.0 == &p.0)
                            {
                                Some(o) => {
                                    let i = &p.1[(format!("{idx}M").len())..(p.1.len() - 1)]
                                        .parse::<usize>().expect("failed to parse patch index");
                                    if p.1.ends_with('K') {
                                        m.set_knob(*i, *o.1);
                                    } else if p.1.ends_with('I') {
                                        mins[*i] = *o.1;
                                    } else {
                                        unreachable!();
                                    }
                                },
                                None => unreachable!(),
                            }
                        }

                        let mouts = m.step(time, &mins);
                        stepped.push(idx.to_owned());
                        for (i, mo) in mouts.iter().enumerate() {
                            outs.insert(format!("{idx}M{i}O"), *mo);
                        }
                    }
                }
            }
        }

        // Play generated audio
        if let Some(audio_out) = &mut self.audio_out {
            let ao = self.patches.iter()
                .filter(|p| p.1 == "AO")
                .filter_map(|p| outs.get(&p.0))
                .sum::<f32>();

            const BUFSIZE: usize = 4096;
            if audio_out.buffer.len() == BUFSIZE {
                let sr = audio_out.config.sample_rate.0;
                let frames = oddio::Frames::from_slice(sr, &audio_out.buffer);
                let signal: oddio::FramesSignal<f32> = oddio::FramesSignal::from(frames);
                let reinhard = oddio::Reinhard::new(signal);
                let fgain = oddio::FixedGain::new(reinhard, -20.0);

                audio_out.mixer_handle
                    .control::<oddio::Mixer<_>, _>()
                    .play(oddio::MonoToStereo::new(fgain));

                audio_out.buffer = Vec::with_capacity(BUFSIZE);
            }

            for _ in 0..DOWNSAMPLE {
                audio_out.buffer.push(ao);
            }
        }

        // Apply feedback patches to knobs
        for (idx, m) in self.modules.iter_mut()
            .filter(|(idx, _)|
                self.patches.iter()
                    .find(|p| p.1.starts_with(&format!("{idx}M")) && p.1.ends_with("K"))
                    .is_some()
            )
        {
            let inpatches: Vec<&Patch> = self.patches.iter()
                .filter(|p| p.1.starts_with(&format!("{idx}M")))
                .collect();
            for p in inpatches {
                match outs.iter()
                    .find(|o| o.0 == &p.0)
                {
                    Some(o) => {
                        let i = &p.1[(format!("{idx}M").len())..(p.1.len() - 1)]
                            .parse::<usize>().expect("failed to parse patch index");
                        if p.1.ends_with('K') {
                            m.set_knob(*i, *o.1);
                        } else if p.1.ends_with('I') {
                            // TODO apply feedback patches to inputs?
                        } else {
                            unreachable!();
                        }
                    },
                    None => {},
                }
            }
        }
    }
    pub fn render(&mut self, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        for (_, m) in &mut self.modules {
            m.render(meshes, q_text, q_mesh);
        }
    }
}
#[derive(Resource, Debug, Clone)]
struct RackHandle(Handle<Rack>);

fn rack_reloader(mut commands: Commands, mut ev_asset: EventReader<AssetEvent<Rack>>, racks: Res<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut state: ResMut<NextState<AppState>>, query: Query<Entity, Or::<(With<CameraComponent>, With<TopModuleComponent>, With<ModuleMeshComponent>)>>) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Modified { handle } => {
                if handle == &h_rack.0 {
                    if let Some(rack) = racks.get(handle) {
                        if rack.modules.iter()
                            .any(|m| {
                                !m.1.is_init()
                            })
                        {
                            match &state.0 {
                                Some(state) if state == &AppState::Loading => return,
                                _ => {},
                            }

                            for ent in &query {
                                if let Some(ent) = commands.get_entity(ent) {
                                    ent.despawn_recursive();
                                }
                            }
                            state.set(AppState::Loading);
                        }
                    }
                }
            },
            _ => {},
        }
    }
}
fn rack_stepper(time: Res<Time>, mut racks: ResMut<Assets<Rack>>, h_rack: ResMut<RackHandle>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        if let Some(audio_out) = &rack.audio_out {
            let t = time.elapsed_seconds_wrapped();
            let steps = audio_out.config.sample_rate.0 as u64 / FRAME_RATE as u64 / DOWNSAMPLE;
            let d = Duration::from_micros(1000 * 1000 / steps / FRAME_RATE as u64).as_secs_f32();
            for i in 0..steps {
                rack.step(t + i as f32 * d);
            }
        } else {
            rack.init_audio();
        }
    }
}
fn rack_render(mut racks: ResMut<Assets<Rack>>, mut meshes: ResMut<Assets<Mesh>>, h_rack: ResMut<RackHandle>, mut q_text: Query<&mut Text, With<ModuleTextComponent>>, mut q_mesh: Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        rack.render(&mut meshes, &mut q_text, &mut q_mesh);
    }
}
