use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::{prelude::*, sprite::Mesh2dHandle};
use bevy::reflect::TypeUuid;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use serde::Deserialize;

use crate::DOWNSAMPLE;
use crate::{patch::Patches, modules::{Module, ModuleTextComponent, ModuleMeshComponent}};

pub struct AudioContextOutput {
    _device: cpal::Device,
    pub(crate) config: cpal::StreamConfig,

    mixer_handle: oddio::Handle<oddio::Mixer<[f32; 2]>>,
    buffer: Vec<f32>,
}
pub struct AudioContextInput {
    _device: cpal::Device,
    _config: cpal::StreamConfig,

    buffer: Arc<Mutex<Vec<f32>>>,
}
pub(crate) struct AudioContext {
    _host: cpal::Host,
    pub(crate) output: AudioContextOutput,
    pub(crate) input: Option<AudioContextInput>,
}
impl std::fmt::Debug for AudioContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AudioContext")
    }
}

#[derive(Deserialize, TypeUuid, Debug)]
#[uuid = "23f4f379-ed3e-4e41-9093-58b4e73ea9a9"]
pub struct Rack {
    #[serde(skip)]
    pub(crate) audio_context: Option<AudioContext>,
    pub modules: HashMap<String, Box<dyn Module>>,
    pub patches: Patches,
}
impl Rack {
    pub(crate) fn init_audio(&mut self) {
        let host = cpal::default_host();
        let out_device = host.default_output_device().expect("no audio output device available");
        let sample_rate = out_device.default_output_config().unwrap().sample_rate();

        let out_config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (out_mixer_handle, out_mixer) = oddio::split(oddio::Mixer::new());

        let out_stream = out_device.build_output_stream(
            &out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = oddio::frame_stereo(data);
                oddio::run(&out_mixer, sample_rate.0, frames);
            },
            |err| {
                error!("{err}");
            },
            None,
        ).unwrap();
        out_stream.play().unwrap();
        unsafe {
            crate::AUDIO_OUTPUT_STREAM = Some(out_stream);
        }

        let input = match host.default_input_device() {
            Some(in_device) => {
                let in_config = cpal::StreamConfig {
                    channels: 1,
                    sample_rate: in_device.default_input_config().unwrap().sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                };

                let in_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(vec![]));
                let inbuf = in_buffer.clone();

                let in_stream = in_device.build_input_stream(
                    &in_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = inbuf.lock() {
                            buf.extend(data);
                        } else {
                            error!("dropped audio input");
                        }
                    },
                    |err| {
                        error!("{err}");
                    },
                    None
                ).unwrap();
                in_stream.play().unwrap();
                unsafe {
                    crate::AUDIO_INPUT_STREAM = Some(in_stream);
                }

                Some(AudioContextInput {
                    _device: in_device,
                    _config: in_config,
                    buffer: in_buffer,
                })
            },
            None => None,
        };

        self.audio_context = Some(AudioContext {
            _host: host,
            output: AudioContextOutput {
                _device: out_device,
                config: out_config,
                mixer_handle: out_mixer_handle,
                buffer: vec![],
            },
            input,
        });
    }

    pub fn step(&mut self, time: f32) {
        if self.audio_context.is_none() {
            self.init_audio();
        }

        let mut stepped: Vec<String> = Vec::with_capacity(self.modules.len());
        let mut outs: HashMap<String, f32> = HashMap::with_capacity(self.modules.iter().fold(0, |a, m| a + m.1.outputs()));

        // Step all modules which take no inputs
        for (idx, m) in self.modules.iter_mut()
            .filter(|(idx, m)|
                m.inputs() == 0
                || !self.patches.iter()
                    .any(|p| p.1.starts_with(&format!("{idx}M")))
            )
        {
            let mouts = m.step(time, &vec![0.0; m.inputs()]);
            stepped.push(idx.clone());
            for (i, mo) in mouts.iter().enumerate() {
                outs.insert(format!("{idx}M{i}O"), *mo);
            }
        }

        // Step all other modules
        while stepped.len() < self.modules.len() {
            for (idx, m) in &mut self.modules {
                if !stepped.contains(idx) {
                    let inpatches: Vec<(&str, &str)> = self.patches.iter()
                        .filter(|p| p.1.starts_with(&format!("{idx}M")))
                        .collect();
                    if inpatches.iter()
                        .all(|p| {
                            outs.iter()
                                .any(|o| o.0 == &p.0)
                        })
                    {
                        let mut mins = vec![0.0; m.inputs()];

                        for p in inpatches {
                            match outs.iter()
                                .find(|o| o.0 == &p.0)
                            {
                                Some(o) => {
                                    let i = &p.1[(format!("{idx}M").len())..(p.1.len() - 1)]
                                        .parse::<usize>().expect("failed to parse patch input index");
                                    if p.1.ends_with('K') {
                                        m.set_knob(*i, *o.1);
                                    } else if p.1.ends_with('I') {
                                        mins[*i] = *o.1;
                                    } else {
                                        unreachable!();
                                    }
                                },
                                None => unreachable!(),
                            }
                        }

                        let mouts = m.step(time, &mins);
                        stepped.push(idx.clone());
                        for (i, mo) in mouts.iter().enumerate() {
                            outs.insert(format!("{idx}M{i}O"), *mo);
                        }
                    }
                }
            }
        }

        if let Some(audio_context) = &mut self.audio_context {
            // Play generated audio
            let ao: Vec<f32> = self.modules.iter_mut()
                .map(|m| m.1.drain_audio_buffer())
                .fold(vec![], |mut ao, b| {
                    for (i, sample) in b.iter().enumerate() {
                        if i < ao.len() {
                            ao[i] += sample;
                        } else {
                            ao.push(*sample);
                        }
                    }
                    ao
                });

            const BUFSIZE: usize = 4096;
            if audio_context.output.buffer.len() == BUFSIZE {
                let sr = audio_context.output.config.sample_rate.0;
                let frames = oddio::Frames::from_slice(sr, &audio_context.output.buffer);
                let signal: oddio::FramesSignal<f32> = oddio::FramesSignal::from(frames);
                let reinhard = oddio::Reinhard::new(signal);
                let fgain = oddio::FixedGain::new(reinhard, -20.0);

                audio_context.output.mixer_handle
                    .control::<oddio::Mixer<_>, _>()
                    .play(oddio::MonoToStereo::new(fgain));

                audio_context.output.buffer = Vec::with_capacity(BUFSIZE);
            }

            audio_context.output.buffer.extend(
                ao.iter()
                    .flat_map(|sample| std::iter::repeat(*sample).take(DOWNSAMPLE as usize))
            );

            // Consume captured audio
            if let Some(input) = &mut audio_context.input {
                if let Ok(inbuf) = &mut input.buffer.lock() {
                    let buf = inbuf.drain(0..).collect::<Vec<f32>>();
                    for m in &mut self.modules {
                        m.1.extend_audio_buffer(&buf);
                    }
                }
            }
        }


        // Apply feedback patches to knobs
        for (idx, m) in self.modules.iter_mut()
            .filter(|(idx, _)|
                self.patches.iter()
                    .any(|p| p.1.starts_with(&format!("{idx}M")) && p.1.ends_with('K'))
            )
        {
            let inpatches: Vec<(&str, &str)> = self.patches.iter()
                .filter(|p| p.1.starts_with(&format!("{idx}M")))
                .collect();
            for p in inpatches {
                if let Some(o) = outs.iter().find(|o| o.0 == &p.0) {
                    let i = &p.1[(format!("{idx}M").len())..(p.1.len() - 1)]
                        .parse::<usize>().expect("failed to parse patch input index");
                    if p.1.ends_with('K') {
                        m.set_knob(*i, *o.1);
                    } else if p.1.ends_with('I') {
                        // TODO apply feedback patches to inputs?
                    } else {
                        unreachable!();
                    }
                }
            }
        }
    }
    pub fn render(&mut self, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        for m in self.modules.values_mut() {
            m.render(meshes, q_text, q_mesh);
        }
    }
}
#[derive(Resource, Debug, Clone)]
pub struct RackHandle(pub Handle<Rack>);
