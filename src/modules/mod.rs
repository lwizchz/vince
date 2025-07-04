/*!
All synth modules are defined here.

Most modules can be used in either an audio or video setup but some have
specific applications. See the `audio`, `io`, and `video` submodules for the
complete list of synth modules.
*/

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::{Deserialize, de::{Visitor, self}};

use crate::{StepType, MainCameraComponent};

pub mod io;
use io::*;

pub mod info;

pub mod oscilloscope;
pub mod oscillator;
pub mod noise;
pub mod sequencer;
pub mod multi_sequencer;
pub mod envelope_generator;

pub mod scaler;
pub mod multiplier;
pub mod mixer;
pub mod multi_mixer;
pub mod inverter;

pub mod audio;

pub mod video;

pub mod conway;

#[derive(Debug, Clone)]
pub struct MouseClick {
    pub pos: Vec2,
    pub button: MouseButton,
}

#[typetag::deserialize(tag = "type")]
pub trait Module: std::fmt::Debug + ModuleClone + Send + Sync {
    fn init(&mut self, id: usize, ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor));
    fn exit(&mut self);

    fn is_init(&self) -> bool {
        self.id().is_some()
    }
    fn is_large(&self) -> bool {
        false
    }
    fn is_own_window(&self) -> bool {
        false
    }
    fn get_screen_pos(&self, q_child: &Query<&ChildOf, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>) -> Vec2 {
        if let Some(component) = self.component() {
            if let Ok(parent) = q_child.get(component) {
                if let Ok(pos_screen) = q_transform.get(parent.parent()) {
                    return pos_screen.translation().truncate();
                }
            }
        }
        Vec2::ZERO
    }
    fn get_world_pos(&self, q_child: &Query<&ChildOf, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>, q_main_camera: &Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>) -> Vec3 {
        let pos_screen = self.get_screen_pos(q_child, q_transform);
        if pos_screen == Vec2::ZERO {
            return Vec3::ZERO;
        }

        if let Ok(main_camera) = q_main_camera.single() {
            if let Ok(pos_world) = main_camera.0.viewport_to_world(main_camera.1, pos_screen) {
                return Vec3::new(
                    pos_world.origin.x / 2.0 - 320.0,
                    pos_world.origin.y / 2.0 + 100.0,
                    0.0,
                );
            }
        }
        Vec3::ZERO
    }

    fn id(&self) -> Option<usize>;
    fn name(&self) -> Option<String>;
    fn component(&self) -> Option<Entity>;

    fn inputs(&self) -> usize;
    fn outputs(&self) -> usize;
    fn knobs(&self) -> usize;

    fn get_knobs(&self) -> Vec<f32> {
        vec![]
    }
    fn set_knob(&mut self, _i: usize, _val: f32) {}

    fn drain_audio_buffer(&mut self) -> Vec<[f32; 2]> {
        vec![]
    }
    fn extend_audio_buffer(&mut self, _ai: &[f32]) {}

    fn keyboard_input(&mut self, _keys: &Res<ButtonInput<KeyCode>>) {}
    fn mouse_input(&mut self, mouse_buttons: &Res<ButtonInput<MouseButton>>, window: &Window, q_child: &Query<&ChildOf, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>) {
        if let Some(mpos) = window.cursor_position() {
            let screen_pos = self.get_screen_pos(q_child, q_transform);
            let (w, h) = if self.is_large() {
                (660.0, 550.0)
            } else {
                (170.0, 200.0)
            };


            if mpos.x >= screen_pos.x - w/2.0 && mpos.x < screen_pos.x + w/2.0
                && mpos.y >= screen_pos.y - h/2.0 && mpos.y < screen_pos.y + h/2.0
            {
                for &button in mouse_buttons.get_just_released() {
                    self.mouse_click(MouseClick {
                        pos: mpos,
                        button,
                    });
                }
            }
        }
    }
    fn mouse_click(&mut self, _mouse_click: MouseClick) {}
    fn step(&mut self, time: f64, st: StepType, ins: &[f32]) -> Vec<f32>;
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _q_children: &Query<&Children>, _q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {}
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
#[derive(Component, Debug, Clone)]
pub struct ModuleImageWindowComponent;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleIOK {
    #[default]
    None,
    Input(usize),
    Output(usize),
    Knob(usize),
}
impl ModuleIOK {
    pub fn is_none(&self) -> bool {
        matches!(self, ModuleIOK::None)
    }
    pub fn is_input(&self) -> bool {
        matches!(self, ModuleIOK::Input(_))
    }
    pub fn is_output(&self) -> bool {
        matches!(self, ModuleIOK::Output(_))
    }
    pub fn is_knob(&self) -> bool {
        matches!(self, ModuleIOK::Knob(_))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleKey {
    pub id: usize,
    pub iok: ModuleIOK,
}
struct ModuleKeyVisitor;
impl<'de> Visitor<'de> for ModuleKeyVisitor {
    type Value = ModuleKey;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a module in the format \"XM[Y{I, O, K}]\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Some((id, iok)) = v.split_once('M') {
            let id = id.parse::<usize>()
                .ok().ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(id), &"an ID string parsable as a usize"))?;

            match iok.get(..(iok.len().saturating_sub(1))) {
                Some(iok) if !iok.is_empty() => {
                    let iok = iok.parse::<usize>()
                        .ok().ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(iok), &"an IOK string parsable as a usize"))?;
                    let iok = match v.get(v.len()-1..) {
                        Some("I") => ModuleIOK::Input(iok),
                        Some("O") => ModuleIOK::Output(iok),
                        Some("K") => ModuleIOK::Knob(iok),

                        Some(t) => return Err(de::Error::invalid_value(de::Unexpected::Str(t), &"an I, O, or K")),
                        None => return Err(de::Error::invalid_value(de::Unexpected::Other("nothing"), &"an I, O, or K")),
                    };

                    return Ok(ModuleKey {
                        id,
                        iok,
                    });
                },
                Some(_) | None => {
                    return Ok(ModuleKey {
                        id,
                        iok: ModuleIOK::None,
                    });
                },
            }
        }

        Err(de::Error::invalid_value(de::Unexpected::Str(v), &"a module in the format \"XM[Y{I, O, K}]\""))
    }
}
impl<'de> Deserialize<'de> for ModuleKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ModuleKeyVisitor)
    }
}
