// SPDX-License-Identifier: GPL-3.0-only
//! Speech kept rather than played: the pieces that turn a backend's synthesis
//! into one WAV file for the client, for `POST /tts/synthesize`.
//!
//! The text goes through the speak path's own normalizing, chunking and
//! cloned-voice handling ([`SpeechEngine::synthesize_wav`]); what differs is
//! only where the audio goes. [`CollectSink`] gathers one chunk's samples
//! where the speak path's sink hands them to the speakers, and [`Assembler`]
//! joins the chunks the way playback does: one rate and one channel, with
//! each seam faded so two sentences synthesized apart do not click where they
//! meet.
//!
//! [`SpeechEngine::synthesize_wav`]: super::speech::SpeechEngine::synthesize_wav

use anyhow::{Result, bail};
use super_voice_shared::tts::audio::frames::{AudioParams, Frame, FrameKind};

use crate::tts::audio::playback::mixer::{self, DeviceFormat, SeamFade};
use crate::tts::tts_models::v1::SynthesisSink;

/// How long each end of a chunk fades, as playback fades it.
const FADE_MS: u32 = 8;

/// One chunk's audio, decoded.
///
/// Holds back a trailing partial sample, as the speak path's sink does: a
/// backend may end a frame mid-sample, and the straddling bytes belong to the
/// next frame.
#[derive(Debug, Default)]
pub(crate) struct CollectSink {
    params: Option<AudioParams>,
    leftover: Vec<u8>,
    samples: Vec<f32>,
}

impl CollectSink {
    /// The chunk's samples and the format they are in; `None` when the
    /// backend sent no audio.
    pub(crate) fn into_audio(self) -> Option<(Vec<f32>, AudioParams)> {
        let params = self.params?;
        (!self.samples.is_empty()).then_some((self.samples, params))
    }
}

impl SynthesisSink for CollectSink {
    fn on_params(&mut self, params: AudioParams) -> Result<()> {
        self.params = Some(params);
        Ok(())
    }

    fn on_frame(&mut self, frame: Frame) -> Result<()> {
        if frame.kind != FrameKind::Audio {
            return Ok(());
        }
        let Some(params) = self.params else {
            bail!("audio frame before response headers were read");
        };
        self.leftover.extend_from_slice(&frame.payload);
        let width = (params.format.bytes_per_sample() * params.channels.max(1) as usize).max(1);
        let usable = self.leftover.len() - (self.leftover.len() % width);
        if usable > 0 {
            let bytes: Vec<u8> = self.leftover.drain(..usable).collect();
            self.samples.extend(params.format.decode(&bytes));
        }
        Ok(())
    }
}

/// The chunks of one synthesis, joined into one mono track at the first
/// chunk's rate.
#[derive(Debug, Default)]
pub(crate) struct Assembler {
    format: Option<DeviceFormat>,
    samples: Vec<f32>,
}

impl Assembler {
    /// Add the next chunk. A chunk at another rate, or in stereo, is brought
    /// to the first one's: one file has one format.
    ///
    /// # Errors
    /// When resampling fails.
    pub(crate) fn push(&mut self, samples: &[f32], params: AudioParams) -> Result<()> {
        let format = *self.format.get_or_insert(DeviceFormat {
            sample_rate: params.sample_rate,
            channels: 1,
        });
        let mut chunk = mixer::prepare(samples, params, format)?;
        SeamFade::new(FADE_MS, format).apply(&mut chunk);
        self.samples.extend_from_slice(&chunk);
        Ok(())
    }

    /// The whole synthesis as a mono 16-bit WAV file; `None` when no chunk
    /// had any audio.
    ///
    /// # Errors
    /// When the WAV cannot be written.
    pub(crate) fn finish(self) -> Result<Option<Vec<u8>>> {
        let Some(format) = self.format else {
            return Ok(None);
        };
        Ok(Some(crate::tts::voices::wav::encode_s16(
            &self.samples,
            format.sample_rate,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super_voice_shared::tts::audio::frames::SampleFormat;

    fn params(sample_rate: u32, channels: u16) -> AudioParams {
        AudioParams {
            sample_rate,
            channels,
            format: SampleFormat::F32Le,
        }
    }

    fn frame(payload: Vec<u8>) -> Frame {
        Frame {
            kind: FrameKind::Audio,
            payload,
        }
    }

    fn read_wav(bytes: &[u8]) -> (hound::WavSpec, Vec<i16>) {
        let reader = hound::WavReader::new(std::io::Cursor::new(bytes)).expect("a WAV");
        let spec = reader.spec();
        let samples = reader.into_samples::<i16>().map(Result::unwrap).collect();
        (spec, samples)
    }

    /// A sample split across two frames is decoded whole, not dropped.
    #[test]
    fn a_sample_split_across_frames_is_kept() {
        let mut sink = CollectSink::default();
        sink.on_params(params(24_000, 1)).unwrap();
        let bytes: Vec<u8> = [0.5_f32, -0.25]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        sink.on_frame(frame(bytes[..5].to_vec())).unwrap();
        sink.on_frame(frame(bytes[5..].to_vec())).unwrap();

        let (samples, got) = sink.into_audio().expect("audio");
        assert_eq!(samples, [0.5, -0.25]);
        assert_eq!(got, params(24_000, 1));
    }

    #[test]
    fn a_chunk_with_no_audio_is_none() {
        let mut sink = CollectSink::default();
        sink.on_params(params(24_000, 1)).unwrap();
        assert!(sink.into_audio().is_none());
    }

    /// Chunks join into one mono file at the first chunk's rate, each as long
    /// as it was: a stereo chunk is folded, a chunk at another rate converted.
    #[test]
    fn chunks_join_into_one_mono_file_at_the_first_rate() {
        let mut out = Assembler::default();
        out.push(&vec![0.5; 2_400], params(24_000, 1)).unwrap();
        out.push(&vec![0.25; 2 * 2_400], params(24_000, 2)).unwrap();
        out.push(&vec![0.5; 4_800], params(48_000, 1)).unwrap();

        let (spec, samples) = read_wav(&out.finish().unwrap().expect("a file"));
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 24_000);
        assert_eq!(spec.bits_per_sample, 16);
        let expected = 3 * 2_400;
        assert!(
            samples.len().abs_diff(expected) < 50,
            "{} samples, about {expected} expected",
            samples.len()
        );
    }

    /// Each chunk starts and ends next to silence, so two sentences meet
    /// without a click.
    #[test]
    fn every_seam_meets_at_silence() {
        let mut out = Assembler::default();
        out.push(&vec![0.8; 2_400], params(24_000, 1)).unwrap();
        out.push(&vec![0.8; 2_400], params(24_000, 1)).unwrap();

        let (_, samples) = read_wav(&out.finish().unwrap().expect("a file"));
        assert!(samples[0].abs() < 2_000, "{}", samples[0]);
        assert!(samples[2_399].abs() < 2_000, "{}", samples[2_399]);
        assert!(samples[2_400].abs() < 2_000, "{}", samples[2_400]);
        assert!(samples[1_200] > 20_000, "the middle is untouched");
    }

    #[test]
    fn no_audio_at_all_is_no_file() {
        assert!(Assembler::default().finish().unwrap().is_none());
    }
}
