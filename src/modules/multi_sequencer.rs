/*!
The `MultiSequencer` module outputs notes from the given sequencers, looping
when done.

## Sequencers
Each sequencer is represented by a subarray whose elements are the sequencer
struct and the duration. The duration is multiplied by the sequencer's total
length.

Note that the child sequencer modules will not be stepped unless activated by
this parent module. Additionally, their outputs cannot be patched except by
patching the parent's outputs.

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
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, sequencer::Sequencer}};

#[derive(Deserialize, Debug, Clone)]
pub struct MultiSequencer {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    sequencers: Vec<(Sequencer, f32)>,
    #[serde(skip)]
    last_seq: Option<String>,
    #[serde(skip)]
    time: f64,
    #[serde(skip)]
    last_time: Option<f64>,
}
#[typetag::deserialize]
impl Module for MultiSequencer {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor)) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} MultiSequencer\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in ["Active\n".to_owned(), "\nChildren:\n".to_owned(), ].iter()
                            .cloned()
                            .chain(
                                self.sequencers.iter()
                                        .enumerate()
                                        .map(|(i, seq)| format!(
                                                "{} x{}\n",
                                                seq.0.name()
                                                    .unwrap_or_else(|| format!("SEQ{i}")),
                                                seq.1,
                                            )
                                        )
                            )
                        {
                            p.spawn((
                                TextSpan::new(t),
                                tfc.0.clone(),
                                tfc.1.clone(),
                            ));
                        }
                    }).id()
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
        3
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, time: f64, st: StepType, ins: &[f32]) -> Vec<f32> {
        let lengths: Vec<f32> = self.sequencers.iter()
            .map(|seq| {
                seq.0.notes.iter()
                    .map(|n| n.2)
                    .sum::<f32>()
                * 60.0 / seq.0.get_knobs()[0]
                * seq.1
            }).collect();

        self.time += time - self.last_time.unwrap_or(time);
        self.time %= lengths.iter().sum::<f32>() as f64;
        self.last_time = Some(time);

        let mut time_left = self.time;
        for (i, seq) in self.sequencers.iter_mut()
            .enumerate()
        {
            time_left -= lengths[i] as f64;
            if time_left < 0.0 {
                self.last_seq = seq.0.name();

                seq.0.time = time_left.rem_euclid(lengths[i] as f64);
                seq.0.last_time = None;
                return seq.0.step(time, st, ins);
            }
        }

        vec![0.0, 0.0, 0.0]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_children: &Query<&Children>, q_textspan: &mut Query<&mut TextSpan>, _q_image: &mut Query<&mut ImageNode, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2d, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            let textspans: Vec<(Entity, String)> = q_children.iter_descendants(*component)
                .filter(|c| q_textspan.contains(*c))
                .zip([
                    if let Some(last_seq) = &self.last_seq {
                        format!("Active: {}\n", last_seq)
                    } else {
                        "Active: None\n".to_string()
                    },
                ]).collect();
            for (c, s) in textspans {
                let mut textspan = q_textspan.get_mut(c).expect("Failed to get textspan");
                **textspan = s;
            }
        }
    }
}
