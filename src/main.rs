/*!

This is a modular synthesizer system built to process both audio and video
signals. It is in early development so please do not expect stability.

Modules are defined in individual files under the [`src/modules`][modules]
directory. Each module can take several inputs, produce several outputs, and
use several "knobs" which are used to adjust module-specific parameters such as
gain or speed.

## Usage

It is recommended to run in release mode but it's no longer required.

```
$ cargo run --release racks/
```

# Racks

Racks consist of modules and the patches between them. They are defined as TOML
files under the `assets/racks` directory which is relative to either the
`CARGO_MANIFEST_DIR` or the built executable. See the provided racks for
details on how to make your own. When a rack file is modified, it will be
hot-reloaded without needing to restart the program.

Basic example rack:

```toml
[modules]
0 = { name = "Audio Out", type = "AudioOut", knobs = [1.0] }

1 = { type = "Oscillator", func = "Sine", knobs = [0.0, 440.0, 1.0, 0.0] }
2 = { type = "Oscilloscope" }

[patches]
1M0O = [ # Take module 1's output 0
    "0M0I", # And patch it here to module 0's input 0
    "2M0I", # And here to module 2's input 0
]
```

Each module is keyed by an index. These indices are not necessarily sequential
which allows for easy commenting out of modules during testing. Each module has
a type which specifies the name of the struct defined in the module source.
Optionally, each module can be named. This name will appear on screen in place
of the module type. The remaining parameters are module-specific so be sure to
read each module's documentation to understand what each one does.

A patch consists of a key defining the output index and an array that lists the
input indices that the given output should be copied to. Each IO index is
specific to a certain module, so it also contains the module index. Patches can
also be created between outputs and knobs. See `racks/rack1.toml` for an
example.

*/

#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![feature(iter_array_chunks)]
#![feature(path_file_prefix)]

#![deny(rust_2018_idioms)]
#![allow(elided_lifetimes_in_paths)]

#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::needless_pass_by_value)]

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::ops::DerefMut;
use std::path::{Path};
use std::sync::RwLock;
use std::sync::{Mutex, atomic::{self, AtomicUsize}};
use std::{time::Duration, cmp};
use std::env;

use bevy::render::render_asset::RenderAssetUsages;
use bevy::{prelude::*, app::AppExit, asset::LoadState, sprite::{MaterialMesh2dBundle, Mesh2dHandle}, window::{PrimaryWindow, WindowResolution, PresentMode, WindowRef, WindowMode, WindowResized}, render::{render_resource::PrimitiveTopology, camera::{RenderTarget, ScalingMode}}};

use bevy_common_assets::toml::TomlAssetPlugin;

pub mod rack;
use rack::{Rack, RackMainHandle};

pub mod patch;
use patch::PatchComponent;

pub mod modules;
use modules::{Module, TopModuleComponent, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent, ModuleImageWindowComponent, ModuleKey, ModuleIOK};

const FRAME_RATE: u16 = 60;

static RACK_DIR_IDX: AtomicUsize = AtomicUsize::new(0);
static RACK_DIR_MAPPING: RwLock<Vec<usize>> = RwLock::new(vec![]);

static CONTINUOUS_TIME: Mutex<Option<f64>> = Mutex::new(None);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Vince Audio-Video Synth".to_string(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        })).add_plugins(TomlAssetPlugin::<Rack>::new(&["toml"]))
        .add_plugins(bevy_framepace::FramepacePlugin)
        .init_state::<AppState>()
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / f64::from(FRAME_RATE)))
        .add_systems(Startup, load_rack)
        .add_systems(Update, setup.run_if(in_state(AppState::Loading)))
        .add_systems(Update, setup_patches.run_if(in_state(AppState::Loaded)))
        .add_systems(Update, (rack_reloader, keyboard_input, mouse_input, window_resize).run_if(in_state(AppState::Ready)))
        .add_systems(FixedUpdate, (rack_stepper, rack_render).run_if(in_state(AppState::Ready)))
        .run();
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    Loading,
    Loaded,
    Ready,
}

#[derive(Component)]
pub struct CameraComponent;
#[derive(Component)]
pub struct MainCameraComponent;

fn load_rack(mut commands: Commands, asset_server: Res<AssetServer>, mut settings_fp: ResMut<bevy_framepace::FramepaceSettings>, mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    settings_fp.limiter = bevy_framepace::Limiter::from_framerate(f64::from(FRAME_RATE));

    // Load rack from config
    let rack_path = if let Some(rack_path) = env::args().nth(1) {
        rack_path
    } else {
        "racks/".to_string()
    };
    let h_rack_main = if Path::new(&rack_path).is_dir() {
        RackMainHandle::Folder(asset_server.load_folder(rack_path.clone()))
    } else {
        RackMainHandle::Single(asset_server.load(rack_path.clone()))
    };
    commands.insert_resource(h_rack_main);

    if let Ok(mut window) = q_window.get_single_mut() {
        window.title = format!("Vince Audio-Video Synth - {rack_path}");
    }
}
fn setup(mut commands: Commands, h_rack_main: Res<RackMainHandle>, mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>, mut state: ResMut<NextState<AppState>>, mut q_window: Query<&mut Window, With<PrimaryWindow>>, mut exit: EventWriter<AppExit>) {
    match h_rack_main.get_load_state(&asset_server) {
        Some(LoadState::Failed) => {
            let rack_path = if let Some(rack_path) = env::args().nth(1) {
                rack_path
            } else {
                "racks/".to_string()
            };
            error!("Invalid file path: {}", rack_path);
            error!("Working directory: {:?}", env::current_dir());
            error!("Check whether you need to enable a feature");
            exit.send(AppExit);
        },
        Some(LoadState::NotLoaded | LoadState::Loading) | None => return,
        Some(LoadState::Loaded) => {},
    }

    if RACK_DIR_MAPPING.read().unwrap().is_empty() {
        if let Ok(mut rdm) = RACK_DIR_MAPPING.write() {
            let mut racks_sorted = racks.iter_mut()
                .map(|(id, r)| {
                    r.path = asset_server.get_path(id).map(|ap| ap.to_string());
                    &r.path
                }).enumerate()
                .collect::<Vec<(usize, &Option<String>)>>();
            racks_sorted.sort_by(|(_, p1), (_, p2)| p1.cmp(p2));
            *rdm = racks_sorted.iter()
                .inspect(|(idx, p)| eprintln!("rack {idx}: {p:?}"))
                .map(|(idx, _)| *idx)
                .collect();
        }
    }

    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        // Init rack info
        let rack_path = if let Some(rack_path) = env::args().nth(1) {
            rack_path
        } else {
            "racks/".to_string()
        };
        let mut window_title = format!("Vince Audio-Video Synth - {rack_path}");
        if let Ok(mut window) = q_window.get_single_mut() {
            if let Some(name) = rack.info.get("name") {
                window_title = format!("Vince Audio-Video Synth - {name} - {rack_path}");
            } else if let Some(path) = &rack.path {
                window_title = format!("Vince Audio-Video Synth - {} - {rack_path}", Path::new(path).file_name().unwrap().to_string_lossy());
            }
            window.title = window_title.clone();
        }
        if !rack.info.is_empty() {
            rack.modules.insert(
                ModuleKey {
                    id: usize::MAX,
                    iok: ModuleIOK::None,
                },
                Box::new(modules::info::Info::new(rack.info.clone())),
            );
        }

        // Main camera
        commands.spawn((
            Camera2dBundle::default(),
            CameraComponent,
            MainCameraComponent,
        ));

        // Module rects
        let ts = TextStyle {
            font_size: 16.0,
            color: Color::WHITE,
            ..default()
        };
        let mut component = commands.spawn(
            NodeBundle {
                style: Style {
                    flex_wrap: FlexWrap::Wrap,
                    align_content: AlignContent::FlexStart,
                    ..default()
                },
                ..default()
            },
        );
        let mut sorted_modules = rack.modules.iter_mut().collect::<Vec<(&ModuleKey, &mut Box<dyn Module>)>>();
        sorted_modules.sort_by(|a, b| {
            if a.0.id == usize::MAX {
                cmp::Ordering::Less
            } else if b.0.id == usize::MAX {
                cmp::Ordering::Greater
            } else {
                a.0.cmp(b.0)
            }
        });
        component.with_children(|parent| {
            for m in &mut sorted_modules {
                if m.1.is_own_window() {
                    continue;
                }

                m.1.init(
                    m.0.id,
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                width: if m.1.is_large() {
                                    Val::Px(660.0)
                                } else {
                                    Val::Px(170.0)
                                },
                                height: if m.1.is_large() {
                                    Val::Px(550.0)
                                } else {
                                    Val::Px(200.0)
                                },
                                margin: UiRect::all(Val::Px(5.0)),
                                padding: UiRect::all(Val::Px(10.0)),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            background_color: Color::DARK_GRAY.into(),
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
        });

        // Init modules which have their own window
        for m in &mut sorted_modules {
            if m.1.is_own_window() {
                let mname = m.1.name()
                    .unwrap_or_else(|| {
                        format!("{:?}", m.1)
                            .split_whitespace()
                            .next()
                            .unwrap()
                            .to_string()
                    });
                let child_window = commands.spawn(
                    Window {
                        title: format!("{} - {}", window_title, mname),
                        resolution: if m.1.is_large() {
                            WindowResolution::new(640.0, 480.0)
                        } else {
                            WindowResolution::new(150.0, 100.0)
                        },
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }
                ).id();
                let _child_camera = commands.spawn((
                    Camera2dBundle {
                        camera: Camera {
                            target: RenderTarget::Window(WindowRef::Entity(child_window)),
                            ..default()
                        },
                        transform: Transform::from_xyz(640.0*m.0.id as f32, 1080.0*2.0, 1.0),
                        projection: OrthographicProjection {
                            scaling_mode: if m.1.is_large() {
                                ScalingMode::Fixed {
                                    width: 640.0,
                                    height: 480.0,
                                }
                            } else {
                                ScalingMode::Fixed {
                                    width: 150.0,
                                    height: 100.0,
                                }
                            },
                            ..default()
                        },
                        ..default()
                    },
                    CameraComponent,
                ));

                m.1.init(
                    m.0.id,
                    commands.entity(child_window).commands().spawn((
                        NodeBundle {
                            style: Style {
                                width: if m.1.is_large() {
                                    Val::Px(660.0)
                                } else {
                                    Val::Px(170.0)
                                },
                                height: if m.1.is_large() {
                                    Val::Px(550.0)
                                } else {
                                    Val::Px(200.0)
                                },
                                margin: UiRect::all(Val::Px(5.0)),
                                padding: UiRect::all(Val::Px(10.0)),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            background_color: Color::DARK_GRAY.into(),
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
        }

        // Setup audio
        rack.init_audio();

        state.set(AppState::Loaded);
    }
}
fn setup_patches(mut commands: Commands, mut racks: ResMut<Assets<Rack>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, q_child: Query<&Parent, With<ModuleComponent>>, q_transform: Query<&GlobalTransform>, q_main_camera: Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>, mut state: ResMut<NextState<AppState>>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        // Patch cables
        let colors = [
            Color::RED,
            Color::ORANGE,
            Color::YELLOW,
            Color::GREEN,
            Color::BLUE,
            Color::PURPLE,
        ];
        let mut sorted_patches = rack.patches.iter().collect::<Vec<(&ModuleKey, &ModuleKey)>>();
        sorted_patches.sort();
        for (i, patch) in sorted_patches.iter().enumerate() {
            let mout_id = ModuleKey {
                id: patch.0.id,
                iok: ModuleIOK::None,
            };
            let min_id = ModuleKey {
                id: patch.1.id,
                iok: ModuleIOK::None,
            };
            if let Some(mout) = rack.modules.get(&mout_id) {
                if let Some(min) = rack.modules.get(&min_id) {
                    let startpos = mout.get_world_pos(&q_child, &q_transform, &q_main_camera) + Vec3::new(50.0, 0.0, 0.0);
                    let endpos = min.get_world_pos(&q_child, &q_transform, &q_main_camera) + Vec3::new(-50.0, 0.0, 0.0);
                    let mut bottom = (startpos.y+endpos.y)/2.0;
                    bottom = bottom.min(startpos.y).min(endpos.y);
                    let midpos = Vec3::new((startpos.x+endpos.x)/2.0, bottom - 50.0 - 5.0 * i as f32, 0.0);

                    let points: Vec<Vec3> = [
                        startpos,
                        startpos.lerp(midpos, 0.5) - Vec3::Y * 10.0,
                        midpos,
                        midpos.lerp(endpos, 0.5) - Vec3::Y * 10.0,
                        endpos,
                    ].iter()
                        .map(|p| *p - startpos)
                        .collect();

                    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::all());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);

                    let _component = commands.spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes.add(mesh).into(),
                            material: materials.add(colors[i % colors.len()]),
                            transform: Transform::from_translation(startpos),
                            ..default()
                        },
                        PatchComponent,
                    ));
                }
            }
        }

        state.set(AppState::Ready);
    }
}

fn rack_reloader(mut commands: Commands, mut ev_asset: EventReader<AssetEvent<Rack>>, mut racks: ResMut<Assets<Rack>>, mut state: ResMut<NextState<AppState>>, q_any: Query<Entity, Or::<(With<CameraComponent>, With<TopModuleComponent>, With<ModuleMeshComponent>, With<ModuleImageWindowComponent>, With<PatchComponent>)>>, q_windows: Query<Entity, (With<Window>, Without<PrimaryWindow>)>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    let (rid, rack) = racks.iter_mut().nth(idx).unwrap();

    for ev in ev_asset.read() {
        if let AssetEvent::Modified { id } = ev {
            if rid == *id {
                if rack.modules.iter()
                    .any(|m| {
                        !m.1.is_init()
                    })
                {
                    if let Some(AppState::Loading) = &state.0 {
                        return;
                    }

                    for ent in &q_any {
                        if let Some(ent) = commands.get_entity(ent) {
                            ent.despawn_recursive();
                        }
                    }

                    for window in &q_windows {
                        if let Some(window) = commands.get_entity(window) {
                            window.despawn_recursive();
                        }
                    }

                    info!("Reloading rack...");

                    state.set(AppState::Loading);
                }
            }
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StepType {
    Key,
    Audio,
    Video,
}
fn continuous_step(time: &Res<Time>, dt: f64, rack: &mut Rack, st: StepType) {
    let t: f64 = match CONTINUOUS_TIME.lock().unwrap().deref_mut() {
        Some(t) => {
            *t += dt;
            *t
        },
        ct @ None => {
            *ct = Some(time.elapsed_seconds_wrapped_f64());
            ct.unwrap()
        },
    };

    rack.step(t, st);
}
fn rack_stepper(time: Res<Time>, mut racks: ResMut<Assets<Rack>>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        if let Some(audio_context) = &rack.audio_context {
            let sr = audio_context.output.config.sample_rate.0 as u64;
            let audio_steps = sr / u64::from(FRAME_RATE);

            match rack.info.get("mode").map(|m| m.as_str()) {
                Some("Key") => {
                    let kdt = Duration::from_micros(1000 * 1000 / u64::from(FRAME_RATE)).as_secs_f64();

                    continuous_step(&time, kdt, rack, StepType::Key);
                },
                Some("Audio") | None => {
                    let adt = Duration::from_micros(1000 * 1000 / sr).as_secs_f64();

                    continuous_step(&time, adt, rack, StepType::Key);
                    for _ in 1..audio_steps {
                        continuous_step(&time, adt, rack, StepType::Audio);
                    }
                },
                Some("Video") => {
                    let video_steps = 4;
                    let vdt = Duration::from_nanos(1000 * 1000 * 1000 / sr / video_steps).as_secs_f64();

                    let mut start_step = 2;
                    continuous_step(&time, vdt, rack, StepType::Key);
                    for _ in 1..audio_steps {
                        continuous_step(&time, vdt, rack, StepType::Audio);
                        for _ in start_step..video_steps {
                            continuous_step(&time, vdt, rack, StepType::Video);
                        }
                        if start_step == 2 {
                            start_step = 1;
                        }
                    }
                },
                Some(m) => panic!("Unknown rack mode: {m}"),
            }
        } else {
            rack.init_audio();
        }
    }
}
fn rack_render(mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut q_text: Query<&mut Text, With<ModuleTextComponent>>, mut q_image: Query<&mut UiImage, With<ModuleImageComponent>>, mut q_mesh: Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        rack.render(&mut images, &mut meshes, &mut q_text, &mut q_image, &mut q_mesh);
    }
}
fn keyboard_input(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>, mut racks: ResMut<Assets<Rack>>, mut q_windows: Query<&mut Window>, q_child_windows: Query<Entity, (With<Window>, Without<PrimaryWindow>)>, q_any: Query<Entity, Or::<(With<CameraComponent>, With<TopModuleComponent>, With<ModuleMeshComponent>, With<ModuleImageWindowComponent>, With<PatchComponent>)>>, mut state: ResMut<NextState<AppState>>, mut exit: EventWriter<AppExit>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        rack.keyboard_input(&keys);

        if keys.just_released(KeyCode::ArrowRight) {
            rack.exit();

            if let Some(AppState::Loading) = &state.0 {
                return;
            }

            for ent in &q_any {
                if let Some(ent) = commands.get_entity(ent) {
                    ent.despawn_recursive();
                }
            }

            for window in &q_child_windows {
                if let Some(window) = commands.get_entity(window) {
                    window.despawn_recursive();
                }
            }

            info!("Loading next rack...");

            RACK_DIR_IDX.fetch_add(1, atomic::Ordering::AcqRel);
            RACK_DIR_IDX.fetch_update(
                atomic::Ordering::Release,
                atomic::Ordering::Acquire,
                |mut idx| {
                    idx %= racks.len();
                    Some(idx)
                }
            ).unwrap();

            state.set(AppState::Loading);
        } else if keys.just_released(KeyCode::ArrowLeft) {
            rack.exit();

            if let Some(AppState::Loading) = &state.0 {
                return;
            }

            for ent in &q_any {
                if let Some(ent) = commands.get_entity(ent) {
                    ent.despawn_recursive();
                }
            }

            for window in &q_child_windows {
                if let Some(window) = commands.get_entity(window) {
                    window.despawn_recursive();
                }
            }

            info!("Loading previous rack...");

            RACK_DIR_IDX.fetch_sub(1, atomic::Ordering::AcqRel);
            RACK_DIR_IDX.fetch_min(racks.len()-1, atomic::Ordering::AcqRel);

            state.set(AppState::Loading);
        } else if keys.just_released(KeyCode::F11) {
            for mut window in &mut q_windows {
                if window.focused {
                    match window.mode {
                        WindowMode::Windowed => {
                            window.mode = WindowMode::BorderlessFullscreen;
                        },
                        _ => {
                            window.mode = WindowMode::Windowed;
                        },
                    }
                    break;
                }
            }
        } else if keys.just_released(KeyCode::Escape) {
            rack.exit();
            exit.send(AppExit);
        }
    }
}
fn mouse_input(mouse_buttons: Res<ButtonInput<MouseButton>>, q_windows: Query<&Window, With<PrimaryWindow>>, mut racks: ResMut<Assets<Rack>>, q_child: Query<&Parent, With<ModuleComponent>>, q_transform: Query<&GlobalTransform>) {
    let rdm = RACK_DIR_MAPPING.read().unwrap();
    let idx = rdm[RACK_DIR_IDX.load(atomic::Ordering::Acquire)];
    if let Some((_, rack)) = racks.iter_mut().nth(idx) {
        rack.mouse_input(&mouse_buttons, q_windows.single(), &q_child, &q_transform);
    }
}
fn window_resize(mut commands: Commands, mut ev_resize: EventReader<WindowResized>, q_windows: Query<&PrimaryWindow>, q_patches: Query<Entity, With<PatchComponent>>, mut state: ResMut<NextState<AppState>>) {
    for ev in ev_resize.read() {
        let WindowResized { window, width: _, height: _ } = ev;
        if q_windows.get(*window).is_ok() {
            if let Some(AppState::Loaded) = &state.0 {
                return;
            }

            for patch in &q_patches {
                if let Some(patch) = commands.get_entity(patch) {
                    patch.despawn_recursive();
                }
            }

            state.set(AppState::Loaded);
        }
    }
}
