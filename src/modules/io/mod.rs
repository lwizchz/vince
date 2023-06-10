/*!
The following I/O modules are defined here: `AudioOut`, `AudioIn`,
`CompositeVideoOut`, `ComponentVideoOut`, `VideoIn`, `FileEncoder`,
`FileDecoder`, `MidiIn`
*/

pub mod audio_out;
pub mod audio_in;

pub mod composite_video_out;
pub mod component_video_out;
#[cfg(feature = "video_in")]
pub mod video_in;

#[cfg(feature = "files")]
pub mod file_encoder;
#[cfg(feature = "files")]
pub mod file_decoder;

#[cfg(feature = "midi")]
pub mod midi_in;
