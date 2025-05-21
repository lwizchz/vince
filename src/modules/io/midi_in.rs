/*!
The `MidiIn` module outputs a signal representing the MIDI input for a given
device.

## Inputs
None

## Outputs
0. The frequency signal from the given device
1. The amplitude signal from the given device
2. Control signal 1
3. Control signal 2
4. Control signal 3
5. Control signal 4
6. Control signal 5
7. Control signal 6
8. Control signal 7
9. Control signal 8

##### Note
If the MIDI port becomes stalled, the outputs will all be [f32::NAN].

## Knobs
None

*/

use std::{sync::{Mutex, Arc}, collections::VecDeque};

use bevy::{prelude::*, ecs::system::EntityCommands, utils::HashMap};

use midly::{live::LiveEvent, num::{u4, u7}, MidiMessage};
use serde::Deserialize;

use midir::{MidiInput, MidiInputPort, MidiInputConnection};

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent}};

#[derive(Default, Clone)]
struct MidiInputContext {
    ports_names_conns: Vec<(MidiInputPort, String, Arc<Mutex<MidiInputConnection<()>>>)>,
    events: Arc<Mutex<VecDeque<(u4, MidiMessage)>>>,
}
impl std::fmt::Debug for MidiInputContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MidiInputContext")
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MidiIn {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(skip)]
    midi_context: MidiInputContext,

    #[serde(skip)]
    notes: Vec<(u7, u7)>,
    #[serde(skip)]
    controllers: HashMap<u7, u7>,
    #[serde(skip)]
    bend: f32,
}
#[typetag::deserialize]
impl Module for MidiIn {
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
                    None => format!("M{id} Midi In\n"),
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

        if self.midi_context.ports_names_conns.is_empty() {
            let mut midi_in = MidiInput::new("Vince MidiIn").expect("Failed to init MIDI Input");
            midi_in.ignore(midir::Ignore::None);

            for (i, in_port) in midi_in.ports()
                .iter().enumerate()
            {
                let in_port_name = midi_in.port_name(in_port)
                    .unwrap_or_else(|msg| panic!("Failed to get MIDI Input name for port with index {}: {}", i, msg));
                let events = self.midi_context.events.clone();
                let conn_in = midi_in.connect(in_port, "vince-midi-in", move |_, message, _| {
                    let event = LiveEvent::parse(message)
                        .unwrap_or_else(|msg| panic!("Failed to parse MIDI event: {:?}: {}", message, msg));
                    match event {
                        LiveEvent::Midi { channel, message } => {
                            if let Ok(mut events) = events.try_lock() {
                                events.push_back((channel, message));
                            }
                        },
                        _ => info!("Unhandled MIDI event: {:?}", event),
                    }
                }, ()).unwrap_or_else(|msg| panic!("Failed to connect to MIDI port with index {}: {}", i, msg));

                self.midi_context.ports_names_conns.push((
                    in_port.clone(),
                    in_port_name,
                    Arc::new(Mutex::new(conn_in)),
                ));

                midi_in = MidiInput::new("Vince MidiIn").expect("Failed to init MIDI Input");
                midi_in.ignore(midir::Ignore::None);
            }

            self.controllers.insert(u7::from(1), u7::from(0));
            self.controllers.insert(u7::from(2), u7::from(0));
            self.controllers.insert(u7::from(3), u7::from(0));
            self.controllers.insert(u7::from(4), u7::from(0));
            self.controllers.insert(u7::from(5), u7::from(0));
            self.controllers.insert(u7::from(6), u7::from(0));
            self.controllers.insert(u7::from(7), u7::from(0));
            self.controllers.insert(u7::from(8), u7::from(0));
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
        10
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, st: StepType, _ins: &[f32]) -> Vec<f32> {
        if st == StepType::Video {
            return vec![f32::NAN; 10];
        }

        if let Ok(mut events) = self.midi_context.events.try_lock() {
            let (mut note_key, mut note_depth) = self.notes.pop()
                .unwrap_or((u7::from(0), u7::from(0)));

            if let Some((_channel, msg)) = events.pop_front() {
                match msg {
                    MidiMessage::NoteOff { key, vel: _ } => {
                        if note_key == key {
                            note_depth = u7::from(0);
                        } else {
                            self.notes
                                .extract_if(.., |(k, _d)| *k == key)
                                .last();
                        }
                    },
                    MidiMessage::NoteOn { key, vel } => {
                        if note_key > 0 && note_depth > 0 && note_key != key {
                            self.notes.push((note_key, note_depth));
                        }
                        note_key = key;
                        note_depth = vel;
                    },
                    MidiMessage::Controller { controller, value } => {
                        *self.controllers.entry(controller)
                            .or_insert(u7::from(0)) = value;
                    },
                    MidiMessage::PitchBend { bend } => {
                        self.bend = bend.as_f32();
                    },
                    _ => info!("Unhandled MIDI message: {:?}", msg),
                }
            } else if st == StepType::Key {
                // note_depth -= u7::from(1);
            }

            let freq = if note_key > 0 && note_depth > 0 {
                self.notes.push((note_key, note_depth));

                2.0f32.powf((note_key.as_int() as i16 - 69) as f32 / 12.0 + self.bend) * 440.0
            } else {
                0.0
            };

            let u7max: f32 = u7::max_value().as_int() as f32;
            vec![
                freq,
                note_depth.as_int() as f32 / u7max,

                self.controllers.get(&u7::from(1))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(2))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(3))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(4))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(5))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(6))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(7))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
                self.controllers.get(&u7::from(8))
                    .unwrap_or(&u7::from(0))
                    .as_int() as f32 / u7max,
            ]
        } else {
            vec![f32::NAN; 10]
        }
    }
}
