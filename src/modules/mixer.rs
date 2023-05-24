use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent};

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
