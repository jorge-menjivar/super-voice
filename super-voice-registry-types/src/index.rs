// SPDX-License-Identifier: GPL-3.0-only
//! `index.json`, as Super Voice reads it: everything in
//! [`super_engine_spec::index`], with the types that carry product fields
//! bound to Super Voice's [`VoiceIndexModel`] and [`VoiceIndexBackend`].

pub use super_engine_spec::index::*;

pub use crate::product::{VoiceIndexBackend, VoiceIndexModel};

/// Super Voice's registry index.
pub type Index = super_engine_spec::index::Index<VoiceIndexModel, VoiceIndexBackend>;

/// One backend in [`Index`], with what its models can do summed up.
pub type IndexBackend = super_engine_spec::index::IndexBackend<VoiceIndexModel, VoiceIndexBackend>;

/// One model of an [`IndexBackend`], with its `role`.
pub type IndexModel = super_engine_spec::index::IndexModel<VoiceIndexModel>;
