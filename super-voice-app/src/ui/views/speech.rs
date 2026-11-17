// SPDX-License-Identifier: GPL-3.0-only
//! The speech test, on text to speech's Behavior tab: text to speak, and the
//! level of what the daemon is saying.
use cosmic::Element;
use cosmic::iced::widget::row;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, button, settings, text};

use crate::state::SpeakingStatus;
use crate::ui::messages::{Message, SpeechMessage};

/// The speech test's rows, for the Behavior tab's one Test section: status,
/// a text field, and speak/stop with the live output level.
pub(super) fn test_rows<'a>(
    speaking_status: &'a SpeakingStatus,
    test_text: &'a str,
    audio_level: f32,
    is_playing: bool,
) -> Vec<Element<'a, Message>> {
    let status_text = match speaking_status {
        SpeakingStatus::Speaking => {
            if is_playing {
                crate::fl!("speaking-status")
            } else {
                // Queued but nothing has reached the device yet: the daemon
                // answers as soon as an utterance is accepted, which is before
                // the first sample is rendered.
                crate::fl!("starting-status")
            }
        }
        SpeakingStatus::Idle => crate::fl!("not-speaking-status"),
    };

    // Speak is disabled on empty text rather than sending it: the daemon
    // rejects an empty utterance with a coded 400, and a button that only ever
    // produces an error banner is worse than one that is visibly unavailable.
    let action_button = match speaking_status {
        SpeakingStatus::Speaking => button::destructive(crate::fl!("stop"))
            .on_press(Message::Speech(SpeechMessage::StopSpeaking)),
        SpeakingStatus::Idle => {
            let b = button::suggested(crate::fl!("speak"));
            if test_text.trim().is_empty() {
                b
            } else {
                b.on_press(Message::Speech(SpeechMessage::Speak))
            }
        }
    };

    let input = widget::text_input(crate::fl!("type-something-to-hear-it"), test_text)
        .on_input(|s| Message::Speech(SpeechMessage::TestTextChanged(s)))
        .width(Length::Fill);

    let level_widget = row![
        action_button,
        widget::determinate_linear(audio_level.max(if audio_level > 0.0 { 0.1 } else { 0.0 }))
            .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    vec![
        settings::item(crate::fl!("status"), text::body(status_text)).into(),
        settings::flex_item(crate::fl!("text"), input).into(),
        settings::flex_item(crate::fl!("output-level"), level_widget).into(),
    ]
}
