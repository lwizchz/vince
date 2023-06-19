/*!
The `ChromaKey` module takes 6 inputs and outputs the first 3, replacing them
with the second 3 if the Green channel is above the given threshold.

## Inputs
0. First red channel in the range [0.0, 1.0]
1. First green channel in the range [0.0, 1.0]
2. First blue channel in the range [0.0, 1.0]
3. Second red channel in the range [0.0, 1.0]
4. Second green channel in the range [0.0, 1.0]
5. Second blue channel in the range [0.0, 1.0]

## Outputs
0. Red channel in the range [0.0, 1.0]
1. Green channel in the range [0.0, 1.0]
2. Blue channel in the range [0.0, 1.0]

##### Note
If all inputs are [f32::NAN] (unpatched), the output will be [f32::NAN].
Otherwise the signal that's not NAN will be used. If only some channels are
NAN, then NANs are treated as 0.0.

## Knobs
0. Threshold in the range [0.0, 1.0]

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct ChromaKey {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    knobs: [f32; 1],
}
#[typetag::deserialize]
impl Module for ChromaKey {
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
                    None => format!("M{id} ChromaKey\n"),
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
        6
    }
    fn outputs(&self) -> usize {
        3
    }
    fn knobs(&self) -> usize {
        self.knobs.len()
    }

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let threshold = self.knobs[0];

        let mut r0 = ins[0];
        let mut g0 = ins[1];
        let mut b0 = ins[2];

        let mut r1 = ins[3];
        let mut g1 = ins[4];
        let mut b1 = ins[5];

        if r0.is_nan() && g0.is_nan() && b0.is_nan() && r1.is_nan() && g1.is_nan() && b1.is_nan() {
            return vec![f32::NAN; self.outputs()];
        } else if r0.is_nan() && g0.is_nan() && b0.is_nan() {
            return vec![r1, g1, b1];
        } else if r1.is_nan() && g1.is_nan() && b1.is_nan() {
            return vec![r0, g0, b0];
        } else if r0.is_nan() {
            r0 = 0.0;
        } else if g0.is_nan() {
            g0 = 0.0;
        } else if b0.is_nan() {
            b0 = 0.0;
        } else if r1.is_nan() {
            r1 = 0.0;
        } else if g1.is_nan() {
            g1 = 0.0;
        } else if b1.is_nan() {
            b1 = 0.0;
        }

        if g0 >= threshold && r0 < (1.0 - threshold) && b0 < (1.0 - threshold) {
            vec![r1, g1, b1]
        } else {
            vec![r0, g0, b0]
        }
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Threshold: {}\n", self.knobs[0]);
            }
        }
    }
}
