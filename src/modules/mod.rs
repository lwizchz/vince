use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use crate::MainCameraComponent;

pub mod audio_out;
pub mod audio_in;

pub mod video_out;

pub mod mixer;
pub mod multiplier;
pub mod oscillator;
pub mod oscilloscope;

#[typetag::deserialize(tag = "type")]
pub trait Module: std::fmt::Debug + ModuleClone + Send + Sync {
    fn init(&mut self, id: usize, ec: EntityCommands, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle);
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

    fn drain_audio_buffer(&mut self) -> Vec<f32> {
        vec![]
    }
    fn extend_audio_buffer(&mut self, _ai: &[f32]) {}

    fn step(&mut self, time: f32, ins: &[f32]) -> Vec<f32>;
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
