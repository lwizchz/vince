/*!
The `MultiSampler` module outputs signals from the given samplers, looping
when done.

## Samplers
Each sampler is represented by a subarray whose elements are the sampler
struct and the duration. The duration is multiplied by the sampler's total
length.

Note that the child sampler modules will not be stepped unless activated by
this parent module. Additionally, their outputs cannot be patched except by
patching the parent's outputs.

##### Note
All samplers must have the same amount of samples.

## Inputs
None

## Outputs
0. The signal from the first sample
1. The signal from the second sample
...
N. The signal from the Nth sample

## Knobs
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, audio::sampler::Sampler}};

#[derive(Deserialize, Debug, Clone)]
pub struct MultiSampler {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    samplers: Vec<(Sampler, f32)>,
    #[serde(skip)]
    last_samp: Option<String>,
    #[serde(skip)]
    time: f64,
    #[serde(skip)]
    last_time: Option<f64>,
}
#[typetag::deserialize]
impl Module for MultiSampler {
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
                    None => format!("M{id} MultiSampler\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections(
                            std::iter::once(TextSection::new(name, ts.clone()))
                                .chain(std::iter::once(TextSection::new("Active\n", ts.clone())))
                                .chain(std::iter::once(TextSection::new("\nChildren:\n", ts.clone())))
                                .chain(
                                    self.samplers.iter()
                                        .enumerate()
                                        .map(|(i, samp)| TextSection::new(
                                            format!(
                                                "{} x{}\n",
                                                samp.0.name()
                                                    .unwrap_or_else(|| format!("SAMP{i}")),
                                                samp.1,
                                            ),
                                            ts.clone(),
                                        ))
                                )
                        ),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        for samp in &mut self.samplers {
            samp.0.init_readers();
        }
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
        match self.samplers.first() {
            Some(samp) => samp.0.samples.len(),
            None => 0,
        }
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, time: f64, st: StepType, ins: &[f32]) -> Vec<f32> {
        let lengths: Vec<f32> = self.samplers.iter()
            .map(|samp| {
                samp.0.get_knobs()[1]
                * 60.0 / samp.0.get_knobs()[0]
                * samp.1
            }).collect();

        self.time += time - self.last_time.unwrap_or(time);
        self.time %= lengths.iter().sum::<f32>() as f64;
        self.last_time = Some(time);

        let mut time_left = self.time;
        for (i, samp) in self.samplers.iter_mut()
            .enumerate()
        {
            time_left -= lengths[i] as f64;
            if time_left < 0.0 {
                self.last_samp = samp.0.name();

                samp.0.time = time_left.rem_euclid(lengths[i] as f64);
                samp.0.last_time = None;
                return samp.0.step(time, st, ins);
            }
        }

        vec![0.0, 0.0, 0.0]
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                if let Some(last_samp) = &self.last_samp {
                    text.sections[1].value = format!("Active: {}\n", last_samp);
                } else {
                    text.sections[1].value = "Active: None\n".to_string();
                }
            }
        }
    }
}
