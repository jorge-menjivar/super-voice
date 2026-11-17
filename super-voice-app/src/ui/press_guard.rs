// SPDX-License-Identifier: GPL-3.0-only
//! A control that acts only on a click that started on it.
//!
//! libcosmic's toggler switches on a mouse release over it, whether or not the
//! press began there. A dropdown menu picks its option and closes on the
//! press, so the release that ends the same click lands on whatever the menu
//! covered: picking "XDG Desktop Portal" for Write Method, over the Audio
//! Feedback toggle, switched the sounds off too. [`press_guard`] hands its
//! content a release only when the press that began it landed on the content
//! as well.
//!
//! Every other event passes through unchanged, and the content draws, lays
//! out and reports itself to assistive technology as it would on its own.

use cosmic::iced::advanced::clipboard::DndDestinationRectangles;
use cosmic::iced::advanced::widget::{Operation, Tree, tree};
use cosmic::iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use cosmic::iced::{Event, Length, Rectangle, Size, Vector, touch};
use cosmic::{Element, Renderer, Theme};

/// `content`, acting only on a click that started on it.
pub fn press_guard<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(PressGuard {
        content: content.into(),
    })
}

struct PressGuard<'a, Message> {
    content: Element<'a, Message>,
}

/// Whether the click under way started on the content.
#[derive(Default)]
struct State {
    pressed: bool,
}

/// What an event means to the guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Click {
    /// A press: a click starts.
    Press,
    /// A release: the click ends, and the content acts on it.
    Release,
    /// Anything else.
    Other,
}

fn click_of(event: &Event) -> Click {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => Click::Press,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
            Click::Release
        }
        _ => Click::Other,
    }
}

/// Whether the content sees `click`, and whether a click is under way on it
/// afterwards. `over` is whether the pointer is over the content.
const fn admit(pressed: bool, click: Click, over: bool) -> (bool, bool) {
    match click {
        Click::Press => (true, over),
        // A release ends the click either way; only one that began here
        // reaches the content.
        Click::Release => (pressed, false),
        Click::Other => (true, pressed),
    }
}

impl<Message> Widget<Message, Theme, Renderer> for PressGuard<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let (passes, pressed) = admit(
            state.pressed,
            click_of(event),
            cursor.is_over(layout.bounds()),
        );
        state.pressed = pressed;
        if passes {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    fn drag_destinations(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut DndDestinationRectangles,
    ) {
        self.content.as_widget().drag_destinations(
            &tree.children[0],
            layout,
            renderer,
            dnd_rectangles,
        );
    }

    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        tree: &Tree,
        cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        self.content
            .as_widget()
            .a11y_nodes(layout, &tree.children[0], cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::{Click, admit};

    /// The click the bug was: pressed on a dropdown's menu, released over the
    /// toggle the menu covered. The toggle never sees the release.
    #[test]
    fn a_release_whose_press_was_elsewhere_does_not_reach_the_content() {
        let (passes, pressed) = admit(false, Click::Release, true);

        assert!(!passes);
        assert!(!pressed);
    }

    /// An ordinary click on the content: pressed on it, released on it.
    #[test]
    fn a_click_that_starts_on_the_content_reaches_it() {
        let (_, pressed) = admit(false, Click::Press, true);
        assert!(pressed);

        let (passes, pressed) = admit(pressed, Click::Release, true);

        assert!(passes);
        assert!(!pressed, "the click is over");
    }

    /// A press elsewhere ends any click under way here, so a release after it
    /// is not this content's.
    #[test]
    fn a_press_elsewhere_ends_the_click() {
        let (passes, pressed) = admit(true, Click::Press, false);
        assert!(
            passes,
            "the content still sees the press, as it would anyway"
        );
        assert!(!pressed);

        let (passes, _) = admit(pressed, Click::Release, true);
        assert!(!passes);
    }

    /// Everything but a press and a release passes, whatever the click.
    #[test]
    fn other_events_pass() {
        assert_eq!(admit(false, Click::Other, false), (true, false));
        assert_eq!(admit(true, Click::Other, true), (true, true));
    }
}
