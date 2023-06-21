/*!
The `Luma` module takes 3 inputs and converts them from RGB data to Luma.

## Inputs
0. Red channel in the range [0.0, 1.0]
1. Green channel in the range [0.0, 1.0]
2. Blue channel in the range [0.0, 1.0]

## Outputs
0. Luma channel in the range [0.0, 1.0]

##### Note
If all inputs are [f32::NAN] (unpatched), the output will be [f32::NAN].
Otherwise NANs are treated as 0.0.

## Knobs
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct Luma {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,
}
#[typetag::deserialize]
impl Module for Luma {
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
                    None => format!("M{id} Luma\n"),
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
        self.id
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        3
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let mut er = ins[0];
        let mut eg = ins[1];
        let mut eb = ins[2];

        if er.is_nan() && eg.is_nan() && eb.is_nan() {
            return vec![f32::NAN];
        }
        if er.is_nan() {
            er = 0.0;
        }
        if eg.is_nan() {
            eg = 0.0;
        }
        if eb.is_nan() {
            eb = 0.0;
        }

        let ey = 0.299 * er + 0.587 * eg + 0.114 * eb;

        vec![ey]
    }
}
