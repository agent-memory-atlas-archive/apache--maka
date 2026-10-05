//! Window roots and presentation-layer plugins.
use crate::{StyledExt, TextSelectionLayer, WindowBorderStyle, window_border};
use gpui::{
    AnyElement, AnyView, App, AppContext, ClipboardItem, Context, Div, Entity, Global,
    InteractiveElement, IntoElement, Keystroke, ParentElement, Render, Stateful, StyleRefinement,
    Styled, Subscription, Window, div,
};
use std::{any::TypeId, rc::Rc};

/// A presentation layer's retained, per-window facilities.
///
/// Register during explicit application initialization, before creating windows.
/// The view renders above application content. Base owns the root regardless of
/// which plugins are registered; Cargo features never select its type.
///
/// On every root render, each plugin participates in three stages:
///
/// 1. [`RootPlugin::prepare`] synchronizes window state before elements are built.
/// 2. [`RootPlugin::style`] supplies defaults for the root surface.
/// 3. [`RootPlugin::decorate`] wraps the completed surface in presentation owned
///    by the plugin.
///
/// The plugin's [`Render`] output is mounted as an overlay above application
/// content. Plugins and their overlays are processed in registration order, so
/// later plugins appear above earlier ones. Notify the plugin entity after its
/// state changes to render the root again; `prepare` and `style` must not notify,
/// because they run during that render.
///
/// Factories are captured when a root is created, so registration affects only
/// future windows.
pub trait RootPlugin: Render + Sized {
    /// Synchronize settings derived from this plugin with the window.
    ///
    /// This runs before the root surface and plugin overlays are built. It is
    /// intended for window-scoped state such as rem size or the active text
    /// selection scope, not for producing elements.
    fn prepare(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// Apply this plugin's default styles directly to the root surface.
    ///
    /// Styles set on the [`Root`] instance are refined onto the surface after
    /// this hook and therefore take precedence over plugin defaults.
    fn style(&self, _surface: &mut Stateful<Div>, _window: &mut Window, _cx: &mut App) {}

    /// Add presentation around the completed root surface.
    ///
    /// This runs after plugin defaults and instance styles have been applied.
    /// Return `surface` unchanged when no outer presentation is needed.
    /// Platform frame mechanics belong to Root and wrap the resulting surface.
    /// `root` provides read-only access to the root and its application view.
    fn decorate(
        &self,
        surface: AnyElement,
        _root: &Root,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        surface
    }
}

type PluginFactory = Rc<dyn Fn(&mut Window, &mut Context<Root>) -> Plugin>;
type Prepare = Rc<dyn Fn(&mut Window, &mut App)>;
type SurfaceStyle = Rc<dyn Fn(&mut Stateful<Div>, &mut Window, &mut App)>;
type Decorate = Rc<dyn Fn(AnyElement, &Root, &mut Window, &mut App) -> AnyElement>;
#[derive(Default)]
struct PluginRegistry(Vec<(TypeId, PluginFactory)>);
impl Global for PluginRegistry {}
struct Plugin {
    view: AnyView,
    prepare: Prepare,
    style: SurfaceStyle,
    decorate: Decorate,
}

/// The window's content and overlay host, independent of any styled component library.
pub struct Root {
    view: AnyView,
    style: StyleRefinement,
    plugins: Vec<Plugin>,
    _keyboard: Subscription,
    _appearance: Subscription,
    window_border_style: WindowBorderStyle,
}

impl Root {
    /// Register a presentation plugin once per application. Re-registering its
    /// type replaces the factory for future windows rather than mounting it twice.
    pub fn register_plugin<V: RootPlugin>(
        cx: &mut App,
        build: fn(&mut Window, &mut Context<V>) -> V,
    ) {
        if !cx.has_global::<PluginRegistry>() {
            cx.set_global(PluginRegistry::default());
        }
        let factory: PluginFactory = Rc::new(move |window, cx| {
            let entity = cx.new(|cx| build(window, cx));
            let root = cx.weak_entity();
            let observed = entity.clone();
            // Plugin construction can enqueue notifications. Observe only after
            // that effect cycle so mounting a Root does not immediately render
            // application content a second time.
            cx.defer(move |cx| {
                cx.observe(&observed, move |_, cx| {
                    let _ = root.update(cx, |_, cx| cx.notify());
                })
                .detach();
            });
            let prepare = entity.clone();
            let style = entity.clone();
            let decorate = entity.clone();
            Plugin {
                view: entity.into(),
                prepare: Rc::new(move |window, cx| {
                    prepare.update(cx, |state, cx| state.prepare(window, cx))
                }),
                style: Rc::new(move |surface, window, cx| {
                    style.update(cx, |state, cx| state.style(surface, window, cx))
                }),
                decorate: Rc::new(move |surface, root, window, cx| {
                    decorate.update(cx, |state, cx| {
                        state.decorate(surface, root, window, cx).into_any_element()
                    })
                }),
            }
        });
        let plugins = &mut cx.global_mut::<PluginRegistry>().0;
        if let Some(entry) = plugins.iter_mut().find(|(id, _)| *id == TypeId::of::<V>()) {
            entry.1 = factory;
        } else {
            plugins.push((TypeId::of::<V>(), factory));
        }
    }

    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        #[cfg(all(target_os = "macos", not(test)))]
        crate::install_window_hit_test_forwarder(window);
        let factories = cx
            .try_global::<PluginRegistry>()
            .map(|e| e.0.clone())
            .unwrap_or_default();
        let window_id = window.window_handle().window_id();
        let owner = cx.weak_entity();
        let keyboard = App::observe_keystrokes(cx, move |event, window, cx| {
            if window.window_handle().window_id() != window_id
                || event.action.is_some()
                || !cx.is_propagating()
                || window.default_prevented()
                || !window
                    .root::<Self>()
                    .flatten()
                    .is_some_and(|current| current.entity_id() == owner.entity_id())
            {
                return;
            }
            let _ = owner.update(cx, |root, cx| {
                root.apply_default(&event.keystroke, window, cx)
            });
        });
        let appearance_owner = cx.weak_entity();
        let appearance = window.observe_window_appearance(move |_, cx| {
            let _ = appearance_owner.update(cx, |_, cx| cx.notify());
        });
        Self {
            view: view.into(),
            _keyboard: keyboard,
            _appearance: appearance,
            window_border_style: WindowBorderStyle::default(),
            style: StyleRefinement::default(),
            plugins: factories
                .into_iter()
                .map(|(_, build)| build(window, cx))
                .collect(),
        }
    }

    /// Update immutable frame values on this original per-window owner.
    pub fn set_window_border_style(&mut self, style: WindowBorderStyle, cx: &mut Context<Self>) {
        if self.window_border_style != style {
            self.window_border_style = style;
            cx.notify();
        }
    }

    /// The original application content entity.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    /// Find a presentation plugin owned by this window.
    pub fn plugin<V: RootPlugin>(&self) -> Option<Entity<V>> {
        self.plugins
            .iter()
            .find_map(|entry| entry.view.clone().downcast::<V>().ok())
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        window
            .root::<Self>()
            .flatten()
            .expect("window must have a Base Root")
            .read(cx)
    }
    pub fn update<R>(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    ) -> R {
        let root = window
            .root::<Self>()
            .flatten()
            .expect("window must have a Base Root");
        root.update(cx, |root, cx| f(root, window, cx))
    }
    // Native defaults run after component bindings and raw bubble decisions,
    // including with no focused descendant. A plugin can veto a default while
    // preserving ancestor observation.
    fn apply_default(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) {
        if window.default_prevented() {
            return;
        }
        let modifiers = key.modifiers;
        let plain_tab = key.key == "tab"
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.platform
            && !modifiers.function;
        let copy_modifier = if cfg!(target_os = "macos") {
            modifiers.platform && !modifiers.control
        } else {
            modifiers.control && !modifiers.platform
        };
        let copy = key.key == "c"
            && copy_modifier
            && !modifiers.alt
            && !modifiers.shift
            && !modifiers.function;
        let consumed = if plain_tab {
            if modifiers.shift {
                self.focus_prev(window, cx);
            } else {
                self.focus_next(window, cx);
            }
            true
        } else {
            copy && self.copy_selection(window, cx)
        };
        if consumed {
            window.prevent_default();
            cx.stop_propagation();
        }
    }

    fn focus_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = crate::active_focus_trap(window, cx) {
            // We're in a focus trap - try to focus next, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_next(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the beginning
                // Find the first focusable element in the trap by continuing to focus_next
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_next(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_next(cx);
    }

    fn focus_prev(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = crate::active_focus_trap(window, cx) {
            // We're in a focus trap - try to focus previous, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_prev(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the end
                // Find the last focusable element in the trap by continuing to focus_prev
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_prev(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_prev(cx);
    }

    fn copy_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let text = crate::TextSelection::selected_text(window, cx)
            .trim()
            .to_string();
        if text.is_empty() {
            return false;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }
}
impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for plugin in &self.plugins {
            (plugin.prepare)(window, cx);
        }
        let mut content = div()
            .id("root")
            .relative()
            .size_full()
            .child(TextSelectionLayer)
            .child(self.view.clone())
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .children(self.plugins.iter().map(|plugin| plugin.view.clone())),
            );
        for plugin in &self.plugins {
            (plugin.style)(&mut content, window, cx);
        }
        let mut content = content.refine_style(&self.style).into_any_element();
        for plugin in &self.plugins {
            content = (plugin.decorate)(content, self, window, cx);
        }
        window_border()
            .style(self.window_border_style.clone())
            .child(content)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    struct Content;
    impl Render for Content {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    struct SelectableContent;
    impl Render for SelectableContent {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child(crate::TextView::markdown(
                "copy-content",
                "quick select value",
            ))
        }
    }

    #[gpui::test]
    fn root_copy_default_preserves_actual_selection_and_obeys_native_veto(cx: &mut TestAppContext) {
        use gpui::{
            Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, VisualTestContext, point, px,
        };
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| SelectableContent);
            Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let position = point(px(10.), px(16.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                crate::TextSelection::selected_text(window, cx).trim(),
                "quick"
            );
            window.blur(cx);
            cx.write_to_clipboard(ClipboardItem::new_string("Original clipboard".into()));
        });
        let veto = Rc::new(std::cell::Cell::new(true));
        let _intercept = cx.update(|window, cx| {
            let window_id = window.window_handle().window_id();
            let veto = veto.clone();
            cx.intercept_keystrokes(move |event, window, _| {
                if window.window_handle().window_id() == window_id
                    && event.keystroke.key == "c"
                    && veto.get()
                {
                    window.prevent_default();
                }
            })
        });
        let copy = if cfg!(target_os = "macos") {
            "cmd-c"
        } else {
            "ctrl-c"
        };
        cx.simulate_keystrokes(copy);
        cx.update(|window, cx| {
            assert!(window.focused(cx).is_none());
            assert_eq!(
                crate::TextSelection::selected_text(window, cx).trim(),
                "quick"
            );
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("Original clipboard")
            );
        });
        veto.set(false);
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-c"
        } else {
            "ctrl-c"
        });
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("quick")
            )
        });
    }

    struct FocusPair {
        first: gpui::FocusHandle,
        second: gpui::FocusHandle,
    }
    impl FocusPair {
        fn new(cx: &mut Context<Self>) -> Self {
            Self {
                first: cx.focus_handle().tab_stop(true),
                second: cx.focus_handle().tab_stop(true),
            }
        }
    }
    impl Render for FocusPair {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().children([
                div()
                    .id("first")
                    .track_focus(&self.first)
                    .tab_stop(true)
                    .h(gpui::px(24.)),
                div()
                    .id("second")
                    .track_focus(&self.second)
                    .tab_stop(true)
                    .h(gpui::px(24.)),
            ])
        }
    }
    #[gpui::test]
    fn root_tab_starts_from_no_focus_and_replaced_root_cannot_apply_defaults(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(FocusPair::new);
            Root::new(content, window, cx)
        });
        let cx: &mut gpui::VisualTestContext = cx;
        cx.run_until_parked();
        let (first, second) = cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let content = root.read(cx).view.clone().downcast::<FocusPair>().unwrap();
            let pair = content.read(cx);
            let handles = (pair.first.clone(), pair.second.clone());
            window.blur(cx);
            handles
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                window.focused(cx).is_none(),
                "blur itself never starts navigation"
            )
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, _| assert!(first.is_focused(window)));
        cx.update(|window, cx| window.blur(cx));
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, _| assert!(second.is_focused(window)));
        cx.update(|window, cx| {
            window.replace_root(cx, |_, cx| FocusPair::new(cx));
            window.blur(cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            assert!(
                window.focused(cx).is_none(),
                "retained old Root cannot act for a replaced native root"
            );
            assert!(
                root.read(cx).view.clone().downcast::<FocusPair>().is_ok(),
                "old entity is genuinely retained"
            );
        });
    }

    struct Layer;
    impl Render for Layer {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    impl RootPlugin for Layer {}
    fn layer(_: &mut Window, _: &mut Context<Layer>) -> Layer {
        Layer
    }

    #[gpui::test]
    fn plugin_registration_is_idempotent_and_state_is_per_window(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            Root::register_plugin(cx, layer);
            Root::register_plugin(cx, layer);
        });
        let mut ids = Vec::new();
        for _ in 0..2 {
            let (root, _) = cx.add_window_view(|window, cx| {
                let content = cx.new(|_| Content);
                Root::new(content, window, cx)
            });
            let id = root.read_with(cx, |root, _| {
                assert_eq!(root.plugins.len(), 1);
                root.plugin::<Layer>().unwrap().entity_id()
            });
            ids.push(id);
        }
        assert_ne!(ids[0], ids[1]);
    }
}

#[cfg(test)]
mod focus_reveal;
