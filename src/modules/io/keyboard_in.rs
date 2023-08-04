/*!
The `KeyboardIn` module outputs two signals representing the key pressed and
it's attack/sustain/release behavior.

## Inputs
None

## Outputs
0. The frequency signal
1. The attack/sustain/release signal

## Knobs
0. Octave in the range [-5.0, 5.0]

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle, utils::HashMap};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Asr {
    Attack,
    Sustain,
    Release,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyboardIn {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    keys: Vec<(KeyCode, Asr)>,

    knobs: [f32; 1],
}
#[typetag::deserialize]
impl Module for KeyboardIn {
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
                    None => format!("M{id} Keyboard In\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("K0\n", ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });
    }
    fn exit(&mut self) {
        self.id = None;
        self.component = None;
        self.children = vec![];
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        0
    }
    fn outputs(&self) -> usize {
        2
    }
    fn knobs(&self) -> usize {
        1
    }

    fn keyboard_input(&mut self, keys: &Res<Input<KeyCode>>) {
        let valid_keys = [
            KeyCode::Z,
            KeyCode::S,
            KeyCode::X,
            KeyCode::D,
            KeyCode::C,

            KeyCode::V,
            KeyCode::G,
            KeyCode::B,
            KeyCode::H,
            KeyCode::N,
            KeyCode::J,
            KeyCode::M,
        ];
        for vk in valid_keys {
            if keys.just_pressed(vk) {
                self.keys.extract_if(|k| k.0 == vk).last();
                self.keys.push((vk, Asr::Attack));
            } else if keys.just_released(vk) {
                for k in &mut self.keys {
                    if k.0 == vk {
                        k.1 = Asr::Release;
                    }
                }
            } else if keys.pressed(vk) {
                for k in &mut self.keys {
                    if k.0 == vk {
                        k.1 = Asr::Sustain;
                    }
                }
            } else {
                self.keys.extract_if(|k| k.0 == vk).last();
            }
        }
    }
    fn step(&mut self, _time: f64, _st: StepType, _ins: &[f32]) -> Vec<f32> {
        let octave = self.knobs[0];

        match self.keys.last_mut() {
            Some(l) => {
                match l.1 {
                    Asr::Attack => {
                        l.1 = Asr::Sustain;
                        vec![get_freq(l.0, octave), 1.0]
                    },
                    Asr::Sustain => {
                        vec![get_freq(l.0, octave), 0.0]
                    },
                    Asr::Release => {
                        let l = self.keys.pop().unwrap();
                        vec![get_freq(l.0, octave), -1.0]
                    }
                }
            },
            None => vec![0.0; 2],
        }
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Octave: {}\n", self.knobs[0]);
            }
        }
    }
}
fn get_freq(k: KeyCode, octave: f32) -> f32 {
    let key_order = {
        let mut key_order = HashMap::new();

        key_order.insert(KeyCode::Z, -9);
        key_order.insert(KeyCode::S, -8);
        key_order.insert(KeyCode::X, -7);
        key_order.insert(KeyCode::D, -6);
        key_order.insert(KeyCode::C, -5);

        key_order.insert(KeyCode::V, -4);
        key_order.insert(KeyCode::G, -3);
        key_order.insert(KeyCode::B, -2);
        key_order.insert(KeyCode::H, -1);
        key_order.insert(KeyCode::N, 0);
        key_order.insert(KeyCode::J, 1);
        key_order.insert(KeyCode::M, 2);

        key_order
    };

    440.0 * 2.0f32.powf(octave as f32 + key_order[&k] as f32 / 12.0)
}
