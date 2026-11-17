// SPDX-License-Identifier: GPL-3.0-only
use cosmic::Element;
use cosmic::widget::{self, settings};
use super_voice_shared::models::notification_method::NotificationMethod;

use super::common::{error_banner, page_layout};
use crate::ui::messages::{IdleUnloadMessage, LanguageMessage, Message, NotificationMethodMessage};

/// The idle times offered, in seconds. The first is what the toggle turns on.
const IDLE_UNLOAD_CHOICES: [u64; 5] = [300, 60, 900, 1800, 3600];

/// Settings page: what both features share — the primary language, how
/// failures are reported, and how long an unused model stays in memory. The
/// recording's sounds are speech to text's own, on its Behavior tab.
pub fn page<'a>(
    app_primary_language: Option<&'a str>,
    notification_method: NotificationMethod,
    idle_unload_secs: u64,
    action_error: Option<&'a str>,
) -> Element<'a, Message> {
    let lang_label = app_primary_language.map_or_else(
        || crate::fl!("automatic"),
        crate::ui::languages::friendly_name,
    );
    let language_section = settings::section().title(crate::fl!("language")).add(
        settings::item::builder(crate::fl!("primary-language"))
            .description(crate::fl!("default-language-for-models-that-support-it"))
            .control(
                widget::button::standard(lang_label).on_press(Message::Language(
                    LanguageMessage::OpenLanguagePicker { model: None },
                )),
            ),
    );

    // Surface a failed save inline (Tier 1 #13) instead of letting the
    // failure flip the whole app to the connection-error page.
    let mut blocks: Vec<Element<'a, Message>> = Vec::new();
    if let Some(message) = action_error {
        blocks.push(error_banner(message));
    }
    blocks.push(language_section.into());
    blocks.push(notifications_section(notification_method));
    blocks.push(memory_section(idle_unload_secs));

    page_layout(crate::fl!("settings"), settings::view_column(blocks))
}

/// Notifications section: how either feature's failures reach the user. One
/// setting for both, so it is here with the others they share.
fn notifications_section(notification_method: NotificationMethod) -> Element<'static, Message> {
    let methods = [
        NotificationMethod::Auto,
        NotificationMethod::Desktop,
        NotificationMethod::Typed,
        NotificationMethod::Off,
    ];
    let method_names: Vec<String> = methods
        .iter()
        .map(|m| crate::i18n::notification_method_name(*m))
        .collect();
    let selected_index = methods.iter().position(|m| *m == notification_method);

    settings::section()
        .title(crate::fl!("notifications"))
        .add(
            settings::item::builder(crate::fl!("failure-notices"))
                .description(crate::fl!("how-recording-and-speech-failures-are-reported"))
                .control(widget::dropdown(
                    method_names,
                    selected_index,
                    move |index| {
                        Message::NotificationMethod(NotificationMethodMessage::Changed(
                            methods[index],
                        ))
                    },
                )),
        )
        .into()
}

/// Memory section: whether a model sleeps when unused (is unloaded, and loads
/// again on its next use), and after how long. One setting for both features'
/// models.
fn memory_section(idle_unload_secs: u64) -> Element<'static, Message> {
    let enabled = idle_unload_secs > 0;
    let mut section = settings::section().title(crate::fl!("memory")).add(
        settings::item::builder(crate::fl!("sleep-when-idle"))
            .description(crate::fl!("sleep-when-idle-description"))
            .control(cosmic::widget::toggler(enabled).on_toggle(|on| {
                Message::IdleUnload(IdleUnloadMessage::Changed(if on {
                    IDLE_UNLOAD_CHOICES[0]
                } else {
                    0
                }))
            })),
    );

    if enabled {
        // A time set outside the app is listed too, so the dropdown always
        // shows the one in force.
        let mut choices = IDLE_UNLOAD_CHOICES.to_vec();
        if !choices.contains(&idle_unload_secs) {
            choices.push(idle_unload_secs);
        }
        choices.sort_unstable();
        let names: Vec<String> = choices.iter().map(|secs| idle_time_name(*secs)).collect();
        let selected = choices.iter().position(|secs| *secs == idle_unload_secs);
        section = section.add(settings::item::builder(crate::fl!("sleep-after")).control(
            widget::dropdown(names, selected, move |index| {
                Message::IdleUnload(IdleUnloadMessage::Changed(choices[index]))
            }),
        ));
    }

    section.into()
}

/// An idle time as the dropdown shows it: whole minutes when it is one, else
/// seconds.
pub(crate) fn idle_time_name(secs: u64) -> String {
    if secs.is_multiple_of(60) {
        let minutes = secs / 60;
        crate::fl!("idle-minutes", minutes = minutes)
    } else {
        crate::fl!("idle-seconds", seconds = secs)
    }
}
