/*!
The `Sequencer` module outputs notes from the given sequence at the given
tempo, looping when done.

## Notes
Each note is represented by a subarray whose elements are the note's frequency,
volume, and duration. The duration is in terms of quarter notes where 1.0
represents the length of a single quarter note.

You may wish to keep a table of note frequencies on hand to make creating
sequences easier.

##### Note
For more complicated sequences, try using the
[MultiSequencer](crate::modules::multi_sequencer) module which can run several
sequencers one after the other.

## Inputs
None

## Outputs
0. The note's frequency
1. The note's volume
2. The note's press/sustain/release according to the below table:
   * If just triggered this frame: 1.0
   * If just released this frame: -1.0
   * Otherwise: 0.0

## Knobs
0. Tempo in the range (0.0, inf)

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent}};

#[derive(Deserialize, Debug, Clone)]
pub struct Sequencer {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    pub(crate) notes: Vec<(f32, f32, f32)>,
    #[serde(skip)]
    last_note: Option<usize>,
    #[serde(skip)]
    pub(crate) time: f64,
    #[serde(skip)]
    pub(crate) last_time: Option<f64>,

    knobs: [f32; 1],
}
#[typetag::deserialize]
impl Module for Sequencer {
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
                    None => format!("M{id} Sequencer\n"),
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
        3
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

    fn step(&mut self, time: f64, _st: StepType, _ins: &[f32]) -> Vec<f32> {
        let tempo = self.knobs[0];
        if tempo == 0.0 {
            return vec![f32::NAN, f32::NAN, f32::NAN];
        }

        let length: f32 = self.notes.iter()
            .map(|n| n.2)
            .sum();

        self.time += time - self.last_time.unwrap_or(time);
        self.time %= length as f64 * 60.0 / tempo as f64;
        self.last_time = Some(time);

        let mut time_left = self.time;
        for (i, n) in self.notes.iter()
            .enumerate()
        {
            time_left -= n.2 as f64 * 60.0 / tempo as f64;
            if time_left < 0.0 {
                let note = match self.last_note {
                    Some(last_note) if last_note == i => {
                        (n.0, n.1, 0.0)
                    },
                    _ if n.1 == 0.0 => {
                        (n.0, n.1, -1.0)
                    },
                    _ => {
                        (n.0, n.1, 1.0)
                    },
                };

                self.last_note = Some(i);

                return vec![
                    note.0,
                    note.1,
                    note.2,
                ];
            }
        }

        vec![0.0, 0.0, 0.0]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[1].value = format!("K0 Tempo: {}\n", self.knobs[0]);
            }
        }
    }
}
