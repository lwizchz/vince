use std::time::Duration;
use std::env;

use bevy::app::AppExit;
use bevy::asset::LoadState;
use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::WindowResolution, sprite::Mesh2dHandle};
use bevy_common_assets::toml::TomlAssetPlugin;

pub mod rack;
use rack::{Rack, RackHandle};

pub mod modules;
use modules::{TopModuleComponent, ModuleTextComponent, ModuleMeshComponent};

#[cfg(debug_assertions)]
const DOWNSAMPLE: u64 = 2;
#[cfg(not(debug_assertions))]
const DOWNSAMPLE: u64 = 1;

const FRAME_RATE: u16 = 60;
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
        .insert_resource(FixedTime::new_from_secs(1.0 / f32::from(FRAME_RATE)))
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

fn rack_reloader(mut commands: Commands, mut ev_asset: EventReader<AssetEvent<Rack>>, racks: Res<Assets<Rack>>, h_rack: ResMut<RackHandle>, mut state: ResMut<NextState<AppState>>, query: Query<Entity, Or::<(With<CameraComponent>, With<TopModuleComponent>, With<ModuleMeshComponent>)>>) {
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
fn rack_stepper(time: Res<Time>, mut racks: ResMut<Assets<Rack>>, h_rack: ResMut<RackHandle>) {
    if let Some(rack) = racks.get_mut(&h_rack.0) {
        if let Some(audio_out) = &rack.audio_out {
            let t = time.elapsed_seconds_wrapped();
            let steps = u64::from(audio_out.config.sample_rate.0) / u64::from(FRAME_RATE) / DOWNSAMPLE;
            let d = Duration::from_micros(1000 * 1000 / steps / u64::from(FRAME_RATE)).as_secs_f32();
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
