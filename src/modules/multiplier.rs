use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent};

#[derive(Deserialize, Debug, Clone)]
pub struct Multiplier {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

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
                        position_type: PositionType::Relative,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} Multiplier\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });
    }

    fn id(&self) -> Option<usize> {
        return self.id;
    }
    fn component(&self) -> Option<Entity> {
        return self.component;
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

    fn step(&mut self, _time: f32, ins: &[f32]) -> Vec<f32> {
        vec![
            ins.iter()
                .product()
        ]
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {

    }
}
