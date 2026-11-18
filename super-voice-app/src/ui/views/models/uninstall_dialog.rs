// SPDX-License-Identifier: GPL-3.0-only

//! The question asked before uninstalling a backend.
//!
//! An uninstall deletes the backend's downloaded models with it, which can be
//! gigabytes that took minutes to fetch, and it used to happen on the one
//! click that opened nothing else.

use cosmic::Element;
use cosmic::widget::{self, button};

use crate::core::app::AppModel;
use crate::state::Stage;
use crate::ui::messages::{Message, ModelsPageMessage};

/// The dialog for the backend the user asked to uninstall, until they answer.
pub fn uninstall_dialog(app: &AppModel) -> Option<Element<'_, Message>> {
    let source = app.models_page.pending_uninstall.as_deref()?;
    let name = app
        .backends
        .iter()
        .find(|b| b.source == source)
        .map_or(source, |b| b.name.as_str());
    let in_use = Stage::ALL
        .iter()
        .any(|stage| app.selected_backend(*stage).as_deref() == Some(source));
    let mut body = crate::fl!("uninstall-backend-body");
    if in_use {
        body.push(' ');
        body.push_str(&crate::fl!("uninstall-backend-in-use"));
    }
    Some(
        widget::dialog()
            .title(crate::fl!("uninstall-backend-question", backend = name))
            .body(body)
            .primary_action(button::destructive(crate::fl!("uninstall")).on_press(
                Message::ModelsPage(ModelsPageMessage::UninstallBackend(source.to_string())),
            ))
            .secondary_action(
                button::standard(crate::fl!("cancel"))
                    .on_press(Message::ModelsPage(ModelsPageMessage::CancelUninstall)),
            )
            .into(),
    )
}
