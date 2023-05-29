#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use std::time::Duration;
use std::env;

use bevy::{prelude::*, app::AppExit, asset::LoadState, sprite::{MaterialMesh2dBundle, Mesh2dHandle}, window::{PrimaryWindow, WindowResolution}, render::{render_resource::PrimitiveTopology}};

use bevy_common_assets::toml::TomlAssetPlugin;

pub mod rack;
use rack::{Rack, RackHandle};

pub mod patch;
use patch::PatchComponent;

pub mod modules;
use modules::{Module, TopModuleComponent, ModuleComponent, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent};

#[cfg(debug_assertions)]
const DOWNSAMPLE: u64 = 2;
#[cfg(not(debug_assertions))]
const DOWNSAMPLE: u64 = 1;

const FRAME_RATE: u16 = 60;
static mut AUDIO_OUTPUT_STREAM: Option<cpal::Stream> = None;
static mut AUDIO_INPUT_STREAM: Option<cpal::Stream> = None;

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
                ..default()
            }),
            ..default()
        })).add_plugin(TomlAssetPlugin::<Rack>::new(&["toml"]))
        .add_plugin(bevy_framepace::FramepacePlugin)
        .add_state::<AppState>()
        .add_startup_system(load_rack)
        .add_system(setup.run_if(in_state(AppState::Loading)))
        .add_system(setup_patches.run_if(in_state(AppState::Loaded)))
        .add_system(rack_reloader.run_if(in_state(AppState::Ready)))
        .add_system(rack_stepper.in_schedule(CoreSchedule::FixedUpdate).run_if(in_state(AppState::Ready)))
        .add_system(rack_render.in_schedule(CoreSchedule::FixedUpdate).run_if(in_state(AppState::Ready)))
        .add_system(bevy::window::close_on_esc)
        .insert_resource(FixedTime::new_from_secs(1.0 / f32::from(FRAME_RATE)))
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
fn setup(mut commands: Commands, h_rack: ResMut<RackHandle>, mut racks: ResMut<Assets<Rack>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>, mut state: ResMut<NextState<AppState>>, mut exit: EventWriter<AppExit>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
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
        component.with_children(|parent| {
            let mut sorted_modules = rack.modules.iter_mut().collect::<Vec<(&String, &mut Box<dyn Module>)>>();
            sorted_modules.sort_by_key(|m| m.0.parse::<usize>().unwrap());
            for m in sorted_modules {
                let id = m.0.parse::<usize>().unwrap();
                m.1.init(
                    id,
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                size: match m.1.is_large() {
                                    true => Size {
                                        width: Val::Px(660.0),
                                        height: Val::Px(550.0),
                                    },
                                    false => Size {
                                        width: Val::Px(170.0),
                                        height: Val::Px(200.0),
                                    },
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
fn setup_patches(mut commands: Commands, racks: Res<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, q_child: Query<&Parent, With<ModuleComponent>>, q_transform: Query<&GlobalTransform>, q_camera: Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>, mut state: ResMut<NextState<AppState>>) {
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
        let mut sorted_patches = rack.patches.iter().collect::<Vec<(&str, &str)>>();
        sorted_patches.sort_by_key(|p| format!("{}{}", p.0, p.1));
        for (i, patch) in sorted_patches.iter().enumerate() {
            if let Some(mout_id) = patch.0.split_once('M') {
                if let Some(min_id) = patch.1.split_once('M') {
                    if let Some(mout) = rack.modules.get(mout_id.0) {
                        if let Some(min) = rack.modules.get(min_id.0) {
                            let startpos = mout.get_pos(&q_child, &q_transform, &q_camera) + Vec3::new(50.0, 0.0, 0.0);
                            let endpos = min.get_pos(&q_child, &q_transform, &q_camera) + Vec3::new(-50.0, 0.0, 0.0);
                            let mut bottom = (startpos.y+endpos.y)/2.0;
                            bottom = bottom.min(startpos.y).min(endpos.y);
                            let midpos = Vec3::new((startpos.x+endpos.x)/2.0, bottom - 50.0, 0.0);

                            let points: Vec<Vec3> = vec![
                                startpos,
                                startpos.lerp(midpos, 0.5) - Vec3::Y * 3.0,
                                midpos,
                                midpos.lerp(endpos, 0.5) - Vec3::Y * 3.0,
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
fn rack_stepper(time: Res<Time>, mut racks: ResMut<Assets<Rack>>, h_rack: ResMut<RackHandle>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        if let Some(audio_context) = &rack.audio_context {
            let t = time.elapsed_seconds_wrapped();
            let audio_steps = u64::from(audio_context.output.config.sample_rate.0) / u64::from(FRAME_RATE) / DOWNSAMPLE;
            let ad = Duration::from_micros(1000 * 1000 / audio_steps / u64::from(FRAME_RATE)).as_secs_f32();

            let video_steps = 209;
            let _vd = Duration::from_nanos(1000 * 1000 * 1000 / audio_steps / video_steps / u64::from(FRAME_RATE)).as_secs_f32();

            rack.step(t, StepType::Key);
            for i in 1..audio_steps {
                rack.step(t + i as f32 * ad, StepType::Audio);
                // for j in 1..video_steps {
                //     rack.step(t + i as f32 * ad + j as f32 * vd, StepType::Video);
                // }
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
