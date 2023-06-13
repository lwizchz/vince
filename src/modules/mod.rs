/*!
All synth modules are defined here.

Most modules can be used in either an audio or video setup but some have
specific applications. See the `audio`, `io`, and `video` submodules for the
complete list of synth modules.
*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::{Deserialize, de::{Visitor, self}};

use crate::{StepType, MainCameraComponent};

pub mod io;
use io::*;

pub mod info;

pub mod oscilloscope;
pub mod oscillator;
pub mod noise;
pub mod sequencer;
pub mod envelope_generator;

pub mod scaler;
pub mod multiplier;
pub mod mixer;
pub mod inverter;

pub mod audio;

pub mod video;

#[typetag::deserialize(tag = "type")]
pub trait Module: std::fmt::Debug + ModuleClone + Send + Sync {
    fn init(&mut self, id: usize, ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle);
    fn exit(&mut self) {}
    fn is_init(&self) -> bool {
        self.id().is_some()
    }
    fn is_large(&self) -> bool {
        false
    }
    fn get_pos(&self, q_child: &Query<&Parent, With<ModuleComponent>>, q_transform: &Query<&GlobalTransform>, q_camera: &Query<(&Camera, &GlobalTransform), With<MainCameraComponent>>) -> Vec3 {
        if let Some(component) = self.component() {
            if let Ok(parent) = q_child.get(component) {
                if let Ok(pos_screen) = q_transform.get(parent.get()) {
                    if let Ok(camera) = q_camera.get_single() {
                        if let Some(pos_world) = camera.0.viewport_to_world(camera.1, pos_screen.translation().truncate()) {
                            return Vec3::from((pos_world.origin.truncate(), 0.0))
                                * Vec3::new(1.0, -1.0, 1.0)
                                + Vec3::new(0.0, -100.0, 0.0);
                        }
                    }
                }
            }
        }
        Vec3::ZERO
    }

    fn id(&self) -> Option<usize>;
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

    fn step(&mut self, time: f64, st: StepType, ins: &[f32]) -> Vec<f32>;
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {}
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
