// SPDX-License-Identifier: GPL-3.0-only
//! What an installed backend can do, as both installed-backend catalogs list
//! it in `supports`: `/backend/list`, and text to speech's own. The names are
//! the ones the registry index lists on its entries
//! ([`supports`](super_voice_registry_types::index::supports)), so a backend
//! reads the same before and after it is installed.

use super_voice_registry_types::index::supports;
use super_voice_registry_types::manifest::Device;

use crate::stt_models::backends::DiscoveredBackend;

/// `backend`'s `supports` list, from its models and the build installed here.
///
/// `gpu` follows `installed_accel`, the frameworks of the build on disk: a
/// backend whose manifest has GPU models but whose install is its CPU build
/// cannot run on a GPU here, and must not say it can. An empty
/// `installed_accel` is no record (a local-directory import, a `wasm` backend,
/// or an install from before the record), and then the models' devices
/// decide, as they do in the index.
pub(crate) fn supports(backend: &DiscoveredBackend, installed_accel: &[String]) -> Vec<String> {
    let any_device = |device: Device| {
        backend
            .models
            .iter()
            .any(|m| m.supported_devices.contains(&device))
    };
    let gpu = if installed_accel.is_empty() {
        any_device(Device::Gpu)
    } else {
        installed_accel.iter().any(|accel| accel != "cpu")
    };
    let context = backend
        .models
        .iter()
        .any(|m| m.product.takes_context(&backend.capabilities.product));
    let cloning = backend.models.iter().any(|m| m.product.clones_voices());
    [
        (supports::GPU, gpu),
        (supports::CPU, any_device(Device::Cpu)),
        (supports::ONLINE, any_device(Device::None)),
        (supports::DICTATION_CONTEXT, context),
        (supports::VOICE_CLONING, cloning),
    ]
    .into_iter()
    .filter(|(_, listed)| *listed)
    .map(|(entry, _)| entry.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::supports;
    use crate::daemon::test_fixtures::{openai_backend, transcription_model};
    use super_voice_registry_types::manifest::{Device, ModelRole, VoiceKind, VoiceModel};

    /// A backend's list follows its models: an online model lists `online`,
    /// and one taking a context and one cloning add their entries.
    #[test]
    fn the_list_follows_the_models() {
        let source = "github.com/x/y";
        let mut speaks = transcription_model(source);
        speaks.product = VoiceModel {
            voice_kinds: vec![VoiceKind::Cloned],
            clone_ref_seconds: Some(10.0),
            ..VoiceModel::new(ModelRole::Synthesis)
        };
        let mut listens = transcription_model(source);
        listens.product.accepts_context = true;
        let backend = openai_backend(source, vec![listens, speaks], None);
        assert_eq!(
            supports(&backend, &[]),
            ["online", "dictation_context", "voice_cloning"]
        );
    }

    /// The installed build decides `gpu`: a CPU-only install does not list it
    /// even when the manifest's models could run on a GPU, and a GPU build
    /// does. With no install record, the models' devices decide.
    #[test]
    fn the_installed_build_decides_gpu() {
        let source = "github.com/x/y";
        let mut local = transcription_model(source);
        local.supported_devices = vec![Device::Gpu, Device::Cpu];
        let backend = openai_backend(source, vec![local], None);
        assert_eq!(supports(&backend, &["cpu".to_string()]), ["cpu"]);
        assert_eq!(
            supports(&backend, &["cuda".to_string(), "cpu".to_string()]),
            ["gpu", "cpu"]
        );
        assert_eq!(supports(&backend, &[]), ["gpu", "cpu"]);
    }
}
