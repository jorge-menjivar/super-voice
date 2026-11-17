// SPDX-License-Identifier: GPL-3.0-only
//! `index.json`, as Super Voice reads it: everything in
//! [`super_engine_spec::index`], with the types that carry product fields
//! bound to Super Voice's [`VoiceIndexModel`].

pub use super_engine_spec::index::*;

pub use crate::product::VoiceIndexModel;

/// The entries of an index entry's `supports` list, and of a backend's in
/// `/backend/list`: what a backend can do, worked out from its models. The
/// engine's three, from the models' devices, and Super Voice's own.
///
/// A client reading the list skips an entry it does not know: entries are
/// added as backends learn to do more, and a newer index must not stop an
/// older client reading the rest.
pub mod supports {
    pub use super_engine_spec::index::supports::{CPU, GPU, ONLINE};

    /// A model is handed the user's dictation context
    /// ([`VoiceIndexModel::accepts_context`](crate::index::VoiceIndexModel::accepts_context)).
    pub const DICTATION_CONTEXT: &str = "dictation_context";
    /// A model speaks in voices the user cloned
    /// ([`VoiceIndexModel::clones_voices`](crate::index::VoiceIndexModel::clones_voices)).
    pub const VOICE_CLONING: &str = "voice_cloning";

    /// Whether `listed` has `entry`. The list may hold entries this build does
    /// not know; they are skipped.
    #[must_use]
    pub fn has(listed: &[String], entry: &str) -> bool {
        listed.iter().any(|l| l == entry)
    }
}

/// Super Voice's registry index.
pub type Index = super_engine_spec::index::Index<VoiceIndexModel>;

/// One backend in [`Index`].
pub type IndexBackend = super_engine_spec::index::IndexBackend<VoiceIndexModel>;

/// One model of an [`IndexBackend`], with its `role`.
pub type IndexModel = super_engine_spec::index::IndexModel<VoiceIndexModel>;
