use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

pub mod mixer;
pub mod multiplier;
pub mod oscillator;
pub mod oscilloscope;

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
