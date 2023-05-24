use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent};

#[derive(Deserialize, Debug, Clone)]
pub struct Multiplier {
    #[serde(default)]
    id: Option<usize>,
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
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            top: Val::Px(5.0),
                            left: Val::Px(5.0),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(format!("M{id} Multiplier\n"), ts.clone()),
                        ]).with_style(Style {
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                top: Val::Px(5.0),
                                left: Val::Px(5.0),
                                ..default()
                            },
                            ..default()
                        }),
                        ModuleTextComponent,
                    )).id()
                );
            });
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
        0
    }

    fn get_knobs(&self) -> Vec<f32> {
        vec![]
    }
    fn set_knob(&mut self, _i: usize, _val: f32) {}

    fn step(&mut self, _time: f32, ins: &[f32]) -> Vec<f32> {
        vec![
            ins.iter()
                .product()
        ]
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, _q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {

    }
}
