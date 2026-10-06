use gpui::{App, Bounds, IntoElement, ParentElement, Pixels, Styled as _, Window, canvas};

use crate::TextSelectionScopeId;

/// Adds selection scopes to native elements and prepaint observation to parents.
pub trait ElementExt: IntoElement + Sized {
    /// Marks this element subtree as belonging to a text-selection scope.
    fn text_selection_scope(self, scope: TextSelectionScopeId) -> impl IntoElement {
        crate::text_selection::text_selection_scope(scope, self)
    }

    /// Invokes `callback` during prepaint with this element's resolved bounds.
    fn on_prepaint<F>(self, callback: F) -> Self
    where
        F: FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
        Self: ParentElement,
    {
        self.child(
            canvas(
                move |bounds, window, cx| callback(bounds, window, cx),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
    }
}

impl<T: IntoElement> ElementExt for T {}
