/*!

This is a modular synthesizer system built to process both audio and video
signals. It is in early development so please do not expect stability.

Modules are defined in individual files under the `src/modules` directory. Each
module can take several inputs, produce several outputs, and use several "knobs"
which are used to adjust module-specific parameters such as gain or speed.

## Usage

It is recommended to run in release mode but it's no longer required.

```
$ cargo run --release racks/rack0.toml
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
#![feature(drain_filter)]
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

use std::{time::Duration, cmp::Ordering};
use std::env;

use bevy::{prelude::*, app::AppExit, asset::LoadState, sprite::{MaterialMesh2dBundle, Mesh2dHandle}, window::{PrimaryWindow, WindowResolution, PresentMode, WindowRef, WindowMode}, render::{render_resource::PrimitiveTopology, camera::{RenderTarget, ScalingMode}}};

use bevy_common_assets::toml::TomlAssetPlugin;

pub mod rack;
use rack::{Rack, RackHandle};

pub mod patch;
use patch::PatchComponent;

pub mod modules;
use modules::{Module, TopModuleComponent, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent, ModuleKey, ModuleIOK};

const FRAME_RATE: u16 = 60;

static mut CONTINUOUS_TIME: Option<f64> = None;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes: true,
            ..default()
        }).set(WindowPlugin {
            primary_window: Some(Window {
                title: "Vince Audio-Video Synth".to_string(),
                resolution: WindowResolution::new(1920.0, 1080.0),
                resizable: false,
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        })).add_plugin(TomlAssetPlugin::<Rack>::new(&["toml"]))
        .add_plugin(bevy_framepace::FramepacePlugin)
        .add_state::<AppState>()
        .insert_resource(FixedTime::new_from_secs(1.0 / f32::from(FRAME_RATE)))
        .add_startup_system(load_rack)
        .add_system(setup.run_if(in_state(AppState::Loading)))
        .add_system(setup_patches.run_if(in_state(AppState::Loaded)))
        .add_system(rack_reloader.run_if(in_state(AppState::Ready)))
        .add_system(rack_stepper.in_schedule(CoreSchedule::FixedUpdate).run_if(in_state(AppState::Ready)))
        .add_system(rack_render.in_schedule(CoreSchedule::FixedUpdate).run_if(in_state(AppState::Ready)))
        .add_system(keyboard_input.run_if(in_state(AppState::Ready)))
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
        "racks/rack0.toml".to_string()
    };
    let h_rack = RackHandle(asset_server.load(rack_path.clone()));
    commands.insert_resource(h_rack);

    if let Ok(mut window) = q_window.get_single_mut() {
        window.title = format!("Vince Audio-Video Synth - {rack_path}");
    }
}
fn setup(mut commands: Commands, h_rack: ResMut<RackHandle>, mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>, mut state: ResMut<NextState<AppState>>, mut q_window: Query<&mut Window, With<PrimaryWindow>>, mut exit: EventWriter<AppExit>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        // Init rack info
        let rack_path = if let Some(rack_path) = env::args().nth(1) {
            rack_path
        } else {
            "racks/rack0.toml".to_string()
        };
        let mut window_title = format!("Vince Audio-Video Synth - {rack_path}");
        if let Some(name) = rack.info.get("name") {
            if let Ok(mut window) = q_window.get_single_mut() {
                window.title = format!("Vince Audio-Video Synth - {name} - {rack_path}");
                window_title = window.title.clone();
            }
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
            font: asset_server.load("fonts/liberation_mono.ttf"),
            font_size: 16.0,
            color: Color::WHITE,
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
                Ordering::Less
            } else if b.0.id == usize::MAX {
                Ordering::Greater
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
                                size: if m.1.is_large() {
                                    Size {
                                        width: Val::Px(660.0),
                                        height: Val::Px(550.0),
                                    }
                                } else {
                                    Size {
                                        width: Val::Px(170.0),
                                        height: Val::Px(200.0),
                                    }
                                },
                                margin: UiRect::all(Val::Px(5.0)),
                                padding: UiRect::all(Val::Px(10.0)),
                                overflow: Overflow::Hidden,
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
                        // resizable: false,
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
                        transform: Transform::from_xyz(640.0*m.0.id as f32, 1080.0*2.0, 0.0),
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
                    UiCameraConfig {
                        show_ui: false,
                    },
                    CameraComponent,
                ));

                m.1.init(
                    m.0.id,
                    commands.entity(child_window).commands().spawn((
                        NodeBundle {
                            style: Style {
                                size: if m.1.is_large() {
                                    Size {
                                        width: Val::Px(660.0),
                                        height: Val::Px(550.0),
                                    }
                                } else {
                                    Size {
                                        width: Val::Px(170.0),
                                        height: Val::Px(200.0),
                                    }
                                },
                                margin: UiRect::all(Val::Px(5.0)),
                                padding: UiRect::all(Val::Px(10.0)),
                                overflow: Overflow::Hidden,
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
    } else if asset_server.get_load_state(&h_rack.0) == LoadState::Failed {
        let rack_path = if let Some(rack_path) = env::args().nth(1) {
            rack_path
        } else {
            "racks/rack0.toml".to_string()
        };
        error!("Invalid file path: {}", rack_path);
        error!("Check whether you need to enable a feature");
        exit.send(AppExit);
    }
}
fn setup_patches(mut commands: Commands, racks: Res<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, q_child: Query<&Parent, With<ModuleComponent>>, q_transform: Query<&GlobalTransform>, q_main_camera: Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>, mut state: ResMut<NextState<AppState>>) {
    if let Some(rack) = racks.get(&h_rack.0) {
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

                    let points: Vec<Vec3> = vec![
                        startpos,
                        startpos.lerp(midpos, 0.5) - Vec3::Y * 10.0,
                        midpos,
                        midpos.lerp(endpos, 0.5) - Vec3::Y * 10.0,
                        endpos,
                    ].iter()
                        .map(|p| *p - startpos)
                        .collect();

                    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);

                    let _component = commands.spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes.add(mesh).into(),
                            material: materials.add(colors[i % colors.len()].into()),
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

fn rack_reloader(mut commands: Commands, mut ev_asset: EventReader<AssetEvent<Rack>>, racks: Res<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut state: ResMut<NextState<AppState>>, query: Query<Entity, Or::<(With<CameraComponent>, With<TopModuleComponent>, With<ModuleMeshComponent>, With<PatchComponent>)>>) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Modified { handle } = ev {
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
    let t = unsafe {
        match &mut CONTINUOUS_TIME {
            Some(t) => {
                *t += dt;
                *t
            },
            None => {
                CONTINUOUS_TIME = Some(time.elapsed_seconds_wrapped_f64());
                CONTINUOUS_TIME.unwrap()
            },
        }
    };

    rack.step(t, st);
}
fn rack_stepper(time: Res<Time>, mut racks: ResMut<Assets<Rack>>, h_rack: ResMut<RackHandle>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
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
fn rack_render(mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, h_rack: ResMut<RackHandle>, mut q_text: Query<&mut Text, With<ModuleTextComponent>>, mut q_image: Query<&mut UiImage, With<ModuleImageComponent>>, mut q_mesh: Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        rack.render(&mut images, &mut meshes, &mut q_text, &mut q_image, &mut q_mesh);
    }
}
fn keyboard_input(keys: Res<Input<KeyCode>>, mut racks: ResMut<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut q_windows: Query<&mut Window>, mut exit: EventWriter<AppExit>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        if keys.just_released(KeyCode::F11) {
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
