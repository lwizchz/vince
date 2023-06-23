/*!
The `Mixer` module takes up to 8 inputs and adds them together, applying a
separate gain to each.

## Inputs
0. First signal
1. Second signal
...
7. Eighth signal

## Outputs
0. The combined signal

## Knobs
0. Gain for Input 0 in the range [0.0, inf)
1. Gain for Input 1 in the range [0.0, inf)
...
7. Gain for Input 7 in the range [0.0, inf)

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct Mixer {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    knobs: [f32; 8],
}
#[typetag::deserialize]
impl Module for Mixer {
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
                    None => format!("M{id} Mixer\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new("K0\n", ts.clone()),
                            TextSection::new("K1\n", ts.clone()),
                            TextSection::new("K2\n", ts.clone()),
                            TextSection::new("K3\n", ts.clone()),
                            TextSection::new("K4\n", ts.clone()),
                            TextSection::new("K5\n", ts.clone()),
                            TextSection::new("K6\n", ts.clone()),
                            TextSection::new("K7\n", ts),
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
        8
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

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        vec![
            ins.iter()
                .map(|inp| {
                    if inp.is_nan() {
                        0.0
                    } else {
                        *inp
                    }
                }).zip(self.knobs.iter())
                .map(|(inp, gain)| inp * gain)
                .sum()
        ]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Gain 1: {}\n", self.knobs[0]);
                text.sections[2].value = format!("K1 Gain 2: {}\n", self.knobs[1]);
                text.sections[3].value = format!("K2 Gain 3: {}\n", self.knobs[2]);
                text.sections[4].value = format!("K3 Gain 4: {}\n", self.knobs[3]);
                text.sections[5].value = format!("K4 Gain 5: {}\n", self.knobs[4]);
                text.sections[6].value = format!("K5 Gain 6: {}\n", self.knobs[5]);
                text.sections[7].value = format!("K6 Gain 7: {}\n", self.knobs[6]);
                text.sections[8].value = format!("K7 Gain 8: {}\n", self.knobs[7]);
            }
        }
    }
}
