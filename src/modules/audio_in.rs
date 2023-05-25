use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::modules::{Module, ModuleComponent, ModuleTextComponent, ModuleMeshComponent};

#[derive(Deserialize, Debug, Clone)]
pub struct AudioIn {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    component: Option<Entity>,
    #[serde(default)]
    children: Vec<Entity>,

    #[serde(default)]
    audio_buffer: Vec<f32>,

    knobs: [f32; 1],
}
#[typetag::deserialize]
impl Module for AudioIn {
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
                    None => format!("M{id} Audio In\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("K0\n".to_string(), ts),
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
        0
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        self.knobs.len()
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
    }

    fn extend_audio_buffer(&mut self, ai: &[f32]) {
        self.audio_buffer.extend(ai);
    }

    fn step(&mut self, _time: f32, _ins: &[f32]) -> Vec<f32> {
        if !self.audio_buffer.is_empty() {
            vec![self.audio_buffer.remove(0) * self.knobs[0]]
        } else {
            vec![0.0]
        }
    }
    fn render(&mut self, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Gain: {}\n", self.knobs[0]);
            }
        }
    }
}
