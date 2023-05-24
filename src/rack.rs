use std::collections::HashMap;

use bevy::{prelude::*, sprite::Mesh2dHandle};
use bevy::reflect::TypeUuid;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use serde::Deserialize;

use crate::modules::{Module, ModuleTextComponent, ModuleMeshComponent};

#[derive(Deserialize, Debug, Clone)]
pub struct Patch(String, String);

pub struct AudioOut {
    _host: cpal::Host,
    _device: cpal::Device,
    pub(crate) config: cpal::StreamConfig,

    mixer_handle: oddio::Handle<oddio::Mixer<[f32; 2]>>,
    buffer: Vec<f32>,
}
impl std::fmt::Debug for AudioOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AudioOut")
    }
}

#[derive(Deserialize, TypeUuid, Debug)]
#[uuid = "23f4f379-ed3e-4e41-9093-58b4e73ea9a9"]
pub struct Rack {
    #[serde(skip)]
    pub(crate) audio_out: Option<AudioOut>,

    pub modules: HashMap<String, Box<dyn Module>>,
    pub patches: Vec<Patch>,
}
impl Rack {
    #[must_use]
    pub fn new(mut mods: Vec<Box<dyn Module>>, pats: &[(&str, &str)]) -> Self {
        Self {
            audio_out: None,

            modules: mods.drain(0..)
                .enumerate()
                .map(|(i, m)| (format!("{i}"), m.clone()))
                .collect(),
            patches: pats.iter()
                .map(|(p0, p1)| Patch((*p0).to_string(), (*p1).to_string()))
                .collect(),
        }
    }

    pub(crate) fn init_audio(&mut self) {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no audio output device available");
        let sample_rate = device.default_output_config().unwrap().sample_rate();

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (mixer_handle, mixer) = oddio::split(oddio::Mixer::new());

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = oddio::frame_stereo(data);
                oddio::run(&mixer, sample_rate.0, frames);
            },
            move |err| {
                error!("{err}");
            },
            None,
        ).unwrap();
        stream.play().unwrap();
        unsafe {
            crate::AUDIO_STREAM = Some(stream);
        }

        self.audio_out = Some(AudioOut {
            _host: host,
            _device: device,
            config,
            mixer_handle,
            buffer: vec![],
        });
    }

    pub fn step(&mut self, time: f32) {
        if self.audio_out.is_none() {
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
                    let inpatches: Vec<&Patch> = self.patches.iter()
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
                                        .parse::<usize>().expect("failed to parse patch index");
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

        // Play generated audio
        if let Some(audio_out) = &mut self.audio_out {
            let ao = self.patches.iter()
                .filter(|p| p.1 == "AO")
                .filter_map(|p| outs.get(&p.0))
                .sum::<f32>();

            const BUFSIZE: usize = 4096;
            if audio_out.buffer.len() == BUFSIZE {
                let sr = audio_out.config.sample_rate.0;
                let frames = oddio::Frames::from_slice(sr, &audio_out.buffer);
                let signal: oddio::FramesSignal<f32> = oddio::FramesSignal::from(frames);
                let reinhard = oddio::Reinhard::new(signal);
                let fgain = oddio::FixedGain::new(reinhard, -20.0);

                audio_out.mixer_handle
                    .control::<oddio::Mixer<_>, _>()
                    .play(oddio::MonoToStereo::new(fgain));

                audio_out.buffer = Vec::with_capacity(BUFSIZE);
            }

            for _ in 0..crate::DOWNSAMPLE {
                audio_out.buffer.push(ao);
            }
        }

        // Apply feedback patches to knobs
        for (idx, m) in self.modules.iter_mut()
            .filter(|(idx, _)|
                self.patches.iter()
                    .any(|p| p.1.starts_with(&format!("{idx}M")) && p.1.ends_with('K'))
            )
        {
            let inpatches: Vec<&Patch> = self.patches.iter()
                .filter(|p| p.1.starts_with(&format!("{idx}M")))
                .collect();
            for p in inpatches {
                if let Some(o) = outs.iter().find(|o| o.0 == &p.0) {
                    let i = &p.1[(format!("{idx}M").len())..(p.1.len() - 1)]
                        .parse::<usize>().expect("failed to parse patch index");
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
