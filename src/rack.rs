use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::{prelude::*, sprite::Mesh2dHandle};
use bevy::reflect::TypeUuid;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use oddio::Signal;
use serde::Deserialize;

use crate::modules::ModuleIOK;
use crate::{StepType, patch::Patches, modules::{ModuleKey, Module, ModuleTextComponent, ModuleMeshComponent, ModuleImageComponent}};

const AUDIO_BUFFER_SIZE: usize = 512;
const AUDIO_STREAM_SIZE: usize = 16384;

static mut AUDIO_OUTPUT_STREAM: Option<cpal::Stream> = None;
static mut AUDIO_INPUT_STREAM: Option<cpal::Stream> = None;

pub struct AudioContextOutput {
    _device: cpal::Device,
    pub(crate) config: cpal::StreamConfig,

    buf_stream_handle: oddio::Handle<oddio::Stream<[f32; 2]>>,
    buffer: Vec<[f32; 2]>,
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
        write!(f, "AudioContext")
    }
}

#[derive(Deserialize, TypeUuid, Debug)]
#[uuid = "23f4f379-ed3e-4e41-9093-58b4e73ea9a9"]
pub struct Rack {
    #[serde(skip)]
    pub(crate) audio_context: Option<AudioContext>,

    pub modules: HashMap<ModuleKey, Box<dyn Module>>,
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

        let (out_buf_stream_handle, out_buf_stream) = oddio::split(oddio::Stream::<[f32; 2]>::new(sample_rate.0, AUDIO_STREAM_SIZE));

        let out_stream = out_device.build_output_stream(
            &out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = oddio::frame_stereo(data);
                oddio::run(&out_buf_stream, sample_rate.0, frames);
            },
            |err| {
                error!("{err}");
            },
            None,
        ).unwrap();
        out_stream.play().unwrap();
        unsafe {
            AUDIO_OUTPUT_STREAM = Some(out_stream);
        }

        let input = match host.default_input_device() {
            Some(in_device) => {
                let in_channels = in_device.default_input_config().unwrap().channels();
                let in_config = cpal::StreamConfig {
                    channels: in_channels,
                    sample_rate: in_device.default_input_config().unwrap().sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                };

                let in_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(vec![]));
                let inbuf = in_buffer.clone();

                let in_stream = if in_channels == 1 {
                    in_device.build_input_stream(
                        &in_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut buf) = inbuf.lock() {
                                buf.extend(data);
                            } else {
                                error!("Rack dropped audio input");
                            }
                        },
                        |err| {
                            error!("{err}");
                        },
                        None
                    ).unwrap()
                } else if in_channels == 2 {
                    in_device.build_input_stream(
                        &in_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut buf) = inbuf.lock() {
                                buf.extend(
                                    data.iter()
                                        .array_chunks::<2>()
                                        .map(|[left, right]| {
                                            (left + right) / 2.0
                                        })
                                );
                            } else {
                                error!("Rack dropped audio input");
                            }
                        },
                        |err| {
                            error!("{err}");
                        },
                        None
                    ).unwrap()
                } else {
                    panic!("Failed to init audio input stream: unsupported number of channels");
                };
                in_stream.play().unwrap();
                unsafe {
                    AUDIO_INPUT_STREAM = Some(in_stream);
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
                buf_stream_handle: out_buf_stream_handle,
                buffer: vec![],
            },
            input,
        });
    }

    pub fn step(&mut self, time: f64, st: StepType) {
        if self.audio_context.is_none() {
            self.init_audio();
        }

        let mut stepped: Vec<usize> = Vec::with_capacity(self.modules.len());
        let mut outs: HashMap<ModuleKey, f32> = HashMap::with_capacity(self.modules.len());

        // Step all modules which take no inputs
        for (k, m) in self.modules.iter_mut()
            .filter(|(k, m)|
                m.inputs() == 0
                || !self.patches.iter()
                    .any(|p| p.1.id == k.id)
            )
        {
            let mouts = m.step(time, st, &vec![0.0; m.inputs()]);
            stepped.push(k.id);
            for (i, mo) in mouts.iter().enumerate() {
                outs.insert(ModuleKey {
                    id: k.id,
                    iok: ModuleIOK::Output(i),
                }, *mo);
            }
        }

        // Step all other modules
        while stepped.len() < self.modules.len() {
            for (k, m) in &mut self.modules {
                if !stepped.contains(&k.id) {
                    let inpatches: Vec<(&ModuleKey, &ModuleKey)> = self.patches.iter()
                        .filter(|p| p.1.id == k.id)
                        .collect();
                    if inpatches.iter()
                        .all(|p| {
                            outs.iter()
                                .any(|o| o.0 == p.0)
                        })
                    {
                        let mut mins = vec![f32::NAN; m.inputs()];

                        for p in inpatches {
                            if let Some(o) = outs.iter().find(|o| o.0 == p.0) {
                                match p.1.iok {
                                    ModuleIOK::Input(i) => mins[i] = *o.1,
                                    ModuleIOK::Knob(i) => {
                                        if !o.1.is_nan() {
                                            m.set_knob(i, *o.1)
                                        }
                                    },
                                    _ => error!("Can't patch an output to another output"),
                                }
                            }
                        }

                        let mouts = m.step(time, st, &mins);
                        stepped.push(k.id);
                        for (i, mo) in mouts.iter().enumerate() {
                            outs.insert(ModuleKey {
                                id: k.id,
                                iok: ModuleIOK::Output(i),
                            }, *mo);
                        }
                    }
                }
            }
        }

        if let Some(audio_context) = &mut self.audio_context {
            // Play generated audio
            let ao: Vec<[f32; 2]> = self.modules.iter_mut()
                .map(|m| m.1.drain_audio_buffer())
                .fold(vec![], |mut ao, b| {
                    for (i, sample) in b.iter().enumerate() {
                        if i < ao.len() {
                            ao[i][0] += sample[0];
                            ao[i][1] += sample[1];
                        } else {
                            ao.push(*sample);
                        }
                    }
                    ao
                });

            audio_context.output.buffer.extend(ao);
            if audio_context.output.buffer.len() == AUDIO_BUFFER_SIZE {
                let sr = audio_context.output.config.sample_rate.0;
                let frames = oddio::Frames::from_slice(sr, &audio_context.output.buffer);
                let signal = oddio::FramesSignal::from(frames);

                let reinhard = oddio::Reinhard::new(signal);
                let fgain = oddio::FixedGain::new(reinhard, -20.0);

                let mut samples = [[0.0; 2]; AUDIO_BUFFER_SIZE];
                fgain.sample(1.0 / sr as f32, &mut samples);
                audio_context.output.buf_stream_handle
                    .control::<oddio::Stream<_>, _>()
                    .write(&samples);

                audio_context.output.buffer = Vec::with_capacity(AUDIO_BUFFER_SIZE);
            }

            // Consume captured audio
            if let Some(input) = &mut audio_context.input {
                if let Ok(inbuf) = &mut input.buffer.lock() {
                    let buf = inbuf.drain(..).collect::<Vec<f32>>();
                    for m in &mut self.modules {
                        m.1.extend_audio_buffer(&buf);
                    }
                }
            }
        }

        // Apply feedback patches to knobs
        for (k, m) in self.modules.iter_mut()
            .filter(|(k, _)|
                self.patches.iter()
                    .any(|p| p.1.id == k.id && p.1.iok.is_knob())
            )
        {
            let inpatches: Vec<(&ModuleKey, &ModuleKey)> = self.patches.iter()
                .filter(|p| p.1.id == k.id)
                .collect();
            for p in inpatches {
                if let Some(o) = outs.iter().find(|o| o.0 == p.0) {
                    match p.1.iok {
                        ModuleIOK::Knob(i) => {
                            if !o.1.is_nan() {
                                m.set_knob(i, *o.1)
                            }
                        },
                        ModuleIOK::Input(_) => {}, // TODO apply feedback patches to inputs?
                        _ => error!("Can't feedback patch an output to another output"),
                    }
                }
            }
        }
    }
    pub fn render(&mut self, images: &mut ResMut<Assets<Image>>, meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        for m in self.modules.values_mut() {
            m.render(images, meshes, q_text, q_image, q_mesh);
        }
    }
    pub fn exit(&mut self) {
        for m in self.modules.values_mut() {
            m.exit();
        }
    }
}
#[derive(Resource, Debug, Clone)]
pub struct RackHandle(pub Handle<Rack>);
