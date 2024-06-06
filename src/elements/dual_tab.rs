use std::cell::{Ref, RefCell};
use std::rc::Rc;

use rootvg::math::Point;
use rootvg::text::glyphon::FontSystem;
use rootvg::text::TextProperties;
use rootvg::PrimitiveGroup;

use crate::event::{ElementEvent, EventCaptureStatus, PointerButton, PointerEvent};
use crate::layout::{Align, Align2, LayoutDirection, Padding};
use crate::math::{Rect, Size, ZIndex};
use crate::style::{Background, BorderStyle, QuadStyle, DEFAULT_ACCENT_COLOR};
use crate::vg::color::{self, RGBA8};
use crate::view::element::{
    Element, ElementBuilder, ElementContext, ElementFlags, ElementHandle, ElementTooltipInfo,
    RenderContext,
};
use crate::view::{ScissorRectID, MAIN_SCISSOR_RECT};
use crate::window::WindowContext;
use crate::CursorIcon;

use super::button::ButtonState;
use super::dual_button::DualButtonStylePart;
use super::dual_label::{DualLabelClipMode, DualLabelLayout};
use super::dual_toggle_button::{DualToggleButtonInner, DualToggleButtonStyle};
use super::tab::IndicatorLinePlacement;

/// The style of a [`DualTab`] element
#[derive(Debug, Clone, PartialEq)]
pub struct DualTabStyle {
    pub toggle_btn_style: DualToggleButtonStyle,

    pub on_indicator_line_width: f32,
    pub on_indicator_line_style: QuadStyle,
    pub on_indicator_line_padding_to_edges: f32,
}

impl Default for DualTabStyle {
    fn default() -> Self {
        let idle_on = DualButtonStylePart {
            left_font_color: color::WHITE,
            right_font_color: color::WHITE,
            back_quad: QuadStyle {
                bg: Background::Solid(RGBA8::new(70, 70, 70, 255)),
                border: BorderStyle::default(),
            },
        };

        let idle_off = DualButtonStylePart {
            left_font_color: RGBA8::new(255, 255, 255, 125),
            right_font_color: RGBA8::new(255, 255, 255, 125),
            back_quad: QuadStyle {
                bg: Background::Solid(color::TRANSPARENT),
                ..idle_on.back_quad
            },
            ..idle_on
        };

        Self {
            toggle_btn_style: DualToggleButtonStyle {
                left_properties: TextProperties::default(),
                right_properties: TextProperties::default(),

                vertical_align: Align::Center,
                left_min_clipped_size: Size::new(5.0, 5.0),
                right_min_clipped_size: Size::new(5.0, 5.0),
                left_padding: Padding::new(6.0, 6.0, 6.0, 6.0),
                right_padding: Padding::new(6.0, 6.0, 6.0, 6.0),

                clip_mode: DualLabelClipMode::default(),
                layout: DualLabelLayout::default(),

                idle_on: idle_on.clone(),
                hovered_on: DualButtonStylePart {
                    back_quad: QuadStyle {
                        ..idle_on.back_quad
                    },
                    ..idle_on
                },
                disabled_on: DualButtonStylePart {
                    left_font_color: RGBA8::new(255, 255, 255, 150),
                    right_font_color: RGBA8::new(255, 255, 255, 150),
                    back_quad: QuadStyle {
                        bg: Background::Solid(RGBA8::new(50, 50, 50, 255)),
                        ..idle_on.back_quad
                    },
                    ..idle_on
                },

                idle_off: idle_off.clone(),
                hovered_off: DualButtonStylePart {
                    back_quad: QuadStyle {
                        bg: Background::Solid(RGBA8::new(50, 50, 50, 255)),
                        ..idle_off.back_quad
                    },
                    ..idle_off
                },
                disabled_off: DualButtonStylePart {
                    left_font_color: RGBA8::new(255, 255, 255, 100),
                    right_font_color: RGBA8::new(255, 255, 255, 100),
                    ..idle_off
                },
            },
            on_indicator_line_width: 3.0,
            on_indicator_line_style: QuadStyle {
                bg: Background::Solid(DEFAULT_ACCENT_COLOR),
                border: BorderStyle::default(),
            },
            on_indicator_line_padding_to_edges: 0.0,
        }
    }
}

pub struct DualTabBuilder<A: Clone + 'static> {
    pub action: Option<A>,
    pub tooltip_message: Option<String>,
    pub tooltip_align: Align2,
    pub toggled: bool,
    pub left_text: String,
    pub right_text: String,
    pub left_text_offset: Point,
    pub right_text_offset: Point,
    pub style: Rc<DualTabStyle>,
    pub on_indicator_line_placement: IndicatorLinePlacement,
    pub z_index: ZIndex,
    pub bounding_rect: Rect,
    pub manually_hidden: bool,
    pub scissor_rect_id: ScissorRectID,
}

impl<A: Clone + 'static> DualTabBuilder<A> {
    pub fn new(style: &Rc<DualTabStyle>) -> Self {
        Self {
            action: None,
            tooltip_message: None,
            tooltip_align: Align2::TOP_CENTER,
            toggled: false,
            left_text: String::new(),
            right_text: String::new(),
            left_text_offset: Point::default(),
            right_text_offset: Point::default(),
            style: Rc::clone(style),
            on_indicator_line_placement: IndicatorLinePlacement::Top,
            z_index: 0,
            bounding_rect: Rect::default(),
            manually_hidden: false,
            scissor_rect_id: MAIN_SCISSOR_RECT,
        }
    }

    pub fn build(self, cx: &mut WindowContext<'_, A>) -> DualTab {
        DualTabElement::create(self, cx)
    }

    pub fn on_toggled_on(mut self, action: A) -> Self {
        self.action = Some(action);
        self
    }

    pub fn tooltip_message(mut self, message: impl Into<String>, align: Align2) -> Self {
        let msg: String = message.into();
        self.tooltip_message = if msg.is_empty() { None } else { Some(msg) };
        self.tooltip_align = align;
        self
    }

    pub const fn toggled(mut self, toggled: bool) -> Self {
        self.toggled = toggled;
        self
    }

    pub fn left_text(mut self, text: impl Into<String>) -> Self {
        self.left_text = text.into();
        self
    }

    pub fn right_text(mut self, text: impl Into<String>) -> Self {
        self.right_text = text.into();
        self
    }

    /// An offset that can be used mainly to correct the position of icon glyphs.
    /// This does not effect the position of the background quad.
    pub const fn left_text_offset(mut self, offset: Point) -> Self {
        self.left_text_offset = offset;
        self
    }

    /// An offset that can be used mainly to correct the position of icon glyphs.
    /// This does not effect the position of the background quad.
    pub const fn right_text_offset(mut self, offset: Point) -> Self {
        self.right_text_offset = offset;
        self
    }

    pub const fn on_indicator_line_placement(mut self, placement: IndicatorLinePlacement) -> Self {
        self.on_indicator_line_placement = placement;
        self
    }

    pub const fn z_index(mut self, z_index: ZIndex) -> Self {
        self.z_index = z_index;
        self
    }

    pub const fn bounding_rect(mut self, rect: Rect) -> Self {
        self.bounding_rect = rect;
        self
    }

    pub const fn hidden(mut self, hidden: bool) -> Self {
        self.manually_hidden = hidden;
        self
    }

    pub const fn scissor_rect(mut self, scissor_rect_id: ScissorRectID) -> Self {
        self.scissor_rect_id = scissor_rect_id;
        self
    }
}

/// A button element with a label.
pub struct DualTabElement<A: Clone + 'static> {
    shared_state: Rc<RefCell<SharedState>>,
    action: Option<A>,
    tooltip_message: Option<String>,
    tooltip_align: Align2,
    on_indicator_line_placement: IndicatorLinePlacement,
}

impl<A: Clone + 'static> DualTabElement<A> {
    pub fn create(builder: DualTabBuilder<A>, cx: &mut WindowContext<'_, A>) -> DualTab {
        let DualTabBuilder {
            action,
            tooltip_message,
            tooltip_align,
            toggled,
            left_text,
            right_text,
            left_text_offset,
            right_text_offset,
            style,
            on_indicator_line_placement,
            z_index,
            bounding_rect,
            manually_hidden,
            scissor_rect_id,
        } = builder;

        let shared_state = Rc::new(RefCell::new(SharedState {
            inner: DualToggleButtonInner::new(
                left_text,
                right_text,
                left_text_offset,
                right_text_offset,
                toggled,
                &style.toggle_btn_style,
                cx.font_system,
            ),
            style,
        }));

        let element_builder = ElementBuilder {
            element: Box::new(Self {
                shared_state: Rc::clone(&shared_state),
                action,
                tooltip_message,
                tooltip_align,
                on_indicator_line_placement,
            }),
            z_index,
            bounding_rect,
            manually_hidden,
            scissor_rect_id,
        };

        let el = cx
            .view
            .add_element(element_builder, cx.font_system, cx.clipboard);

        DualTab { el, shared_state }
    }
}

impl<A: Clone + 'static> Element<A> for DualTabElement<A> {
    fn flags(&self) -> ElementFlags {
        ElementFlags::PAINTS | ElementFlags::LISTENS_TO_POINTER_INSIDE_BOUNDS
    }

    fn on_event(
        &mut self,
        event: ElementEvent,
        cx: &mut ElementContext<'_, A>,
    ) -> EventCaptureStatus {
        match event {
            ElementEvent::CustomStateChanged => {
                cx.request_repaint();
            }
            ElementEvent::Pointer(PointerEvent::Moved { just_entered, .. }) => {
                let mut shared_state = RefCell::borrow_mut(&self.shared_state);
                let SharedState { inner, style } = &mut *shared_state;

                if inner.state() == ButtonState::Disabled {
                    return EventCaptureStatus::NotCaptured;
                }

                cx.cursor_icon = CursorIcon::Pointer;

                if just_entered && self.tooltip_message.is_some() {
                    cx.start_hover_timeout();
                }

                if inner.state() == ButtonState::Idle {
                    let res = inner.set_state(ButtonState::Hovered, &style.toggle_btn_style);

                    if res.needs_repaint {
                        cx.request_repaint();
                    }
                }

                return EventCaptureStatus::Captured;
            }
            ElementEvent::Pointer(PointerEvent::PointerLeft) => {
                let mut shared_state = RefCell::borrow_mut(&self.shared_state);
                let SharedState { inner, style, .. } = &mut *shared_state;

                if inner.state() == ButtonState::Hovered || inner.state() == ButtonState::Down {
                    let res = inner.set_state(ButtonState::Idle, &style.toggle_btn_style);

                    if res.needs_repaint {
                        cx.request_repaint();
                    }

                    return EventCaptureStatus::Captured;
                }
            }
            ElementEvent::Pointer(PointerEvent::ButtonJustPressed { button, .. }) => {
                let mut shared_state = RefCell::borrow_mut(&self.shared_state);
                let SharedState { inner, style, .. } = &mut *shared_state;

                if button == PointerButton::Primary
                    && (inner.state() == ButtonState::Idle || inner.state() == ButtonState::Hovered)
                    && !inner.toggled()
                {
                    let res1 = inner.set_state(ButtonState::Down, &style.toggle_btn_style);
                    let res2 = inner.set_toggled(!inner.toggled());

                    if res1.needs_repaint || res2.needs_repaint {
                        cx.request_repaint();
                    }

                    if let Some(action) = &self.action {
                        cx.send_action(action.clone()).unwrap();
                    }

                    return EventCaptureStatus::Captured;
                }
            }
            ElementEvent::Pointer(PointerEvent::ButtonJustReleased {
                position, button, ..
            }) => {
                let mut shared_state = RefCell::borrow_mut(&self.shared_state);
                let SharedState { inner, style, .. } = &mut *shared_state;

                if button == PointerButton::Primary
                    && (inner.state() == ButtonState::Down || inner.state() == ButtonState::Hovered)
                {
                    let new_state = if cx.is_point_within_visible_bounds(position) {
                        ButtonState::Hovered
                    } else {
                        ButtonState::Idle
                    };

                    let res = inner.set_state(new_state, &style.toggle_btn_style);

                    if res.needs_repaint {
                        cx.request_repaint();
                    }

                    return EventCaptureStatus::Captured;
                }
            }
            ElementEvent::Pointer(PointerEvent::HoverTimeout { .. }) => {
                if let Some(message) = &self.tooltip_message {
                    cx.show_tooltip(ElementTooltipInfo {
                        message: message.clone(),
                        element_bounds: cx.rect(),
                        align: self.tooltip_align,
                    });
                }
            }
            _ => {}
        }

        EventCaptureStatus::NotCaptured
    }

    fn render_primitives(&mut self, cx: RenderContext<'_>, primitives: &mut PrimitiveGroup) {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);
        let SharedState { inner, style, .. } = &mut *shared_state;

        let label_primitives = inner.render_primitives(
            Rect::from_size(cx.bounds_size),
            &style.toggle_btn_style,
            cx.font_system,
        );

        if let Some(quad_primitive) = label_primitives.bg_quad {
            primitives.add(quad_primitive);
        }

        if let Some(text_primitive) = label_primitives.left_text {
            primitives.set_z_index(1);
            primitives.add_text(text_primitive);
        }
        if let Some(text_primitive) = label_primitives.right_text {
            primitives.set_z_index(1);
            primitives.add_text(text_primitive);
        }

        if style.on_indicator_line_width > 0.0
            && !style.on_indicator_line_style.is_transparent()
            && inner.toggled()
        {
            primitives.set_z_index(1);

            let line_rect = match self.on_indicator_line_placement {
                IndicatorLinePlacement::Top => Rect::new(
                    Point::new(style.on_indicator_line_padding_to_edges, 0.0),
                    Size::new(
                        cx.bounds_size.width - (style.on_indicator_line_padding_to_edges * 2.0),
                        style.on_indicator_line_width,
                    ),
                ),
                IndicatorLinePlacement::Bottom => Rect::new(
                    Point::new(
                        style.on_indicator_line_padding_to_edges,
                        cx.bounds_size.height - style.on_indicator_line_width,
                    ),
                    Size::new(
                        cx.bounds_size.width - (style.on_indicator_line_padding_to_edges * 2.0),
                        style.on_indicator_line_width,
                    ),
                ),
                IndicatorLinePlacement::Left => Rect::new(
                    Point::new(0.0, style.on_indicator_line_padding_to_edges),
                    Size::new(
                        style.on_indicator_line_width,
                        cx.bounds_size.height - (style.on_indicator_line_padding_to_edges * 2.0),
                    ),
                ),
                IndicatorLinePlacement::Right => Rect::new(
                    Point::new(
                        cx.bounds_size.width - style.on_indicator_line_width,
                        style.on_indicator_line_padding_to_edges,
                    ),
                    Size::new(
                        style.on_indicator_line_width,
                        cx.bounds_size.height - (style.on_indicator_line_padding_to_edges * 2.0),
                    ),
                ),
            };

            primitives.add(style.on_indicator_line_style.create_primitive(line_rect));
        }
    }
}

/// A handle to a [`DualTabElement`].
pub struct DualTab {
    pub el: ElementHandle,
    shared_state: Rc<RefCell<SharedState>>,
}

struct SharedState {
    inner: DualToggleButtonInner,
    style: Rc<DualTabStyle>,
}

impl DualTab {
    pub fn builder<A: Clone + 'static>(style: &Rc<DualTabStyle>) -> DualTabBuilder<A> {
        DualTabBuilder::new(style)
    }

    /// Returns the size of the padded background rectangle if it were to
    /// cover the entire size of the unclipped text.
    ///
    /// This can be useful to lay out elements that depend on text size.
    pub fn desired_padded_size(&self) -> Size {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);
        let SharedState { inner, style, .. } = &mut *shared_state;

        inner.desired_padded_size(&style.toggle_btn_style)
    }

    /// Returns the size of the unclipped left and right text.
    ///
    /// This can be useful to lay out elements that depend on text size.
    pub fn unclipped_text_size(&self) -> (Size, Size) {
        RefCell::borrow_mut(&self.shared_state)
            .inner
            .unclipped_text_size()
    }

    pub fn set_left_text(&mut self, text: &str, font_system: &mut FontSystem) {
        if RefCell::borrow_mut(&self.shared_state)
            .inner
            .set_left_text(text, font_system)
        {
            self.el.notify_custom_state_change();
        }
    }

    pub fn set_right_text(&mut self, text: &str, font_system: &mut FontSystem) {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);
        let SharedState { inner, style } = &mut *shared_state;

        inner.set_right_text(text, &style.toggle_btn_style, font_system);
        self.el.notify_custom_state_change();
    }

    pub fn left_text<'a>(&'a self) -> Ref<'a, str> {
        Ref::map(RefCell::borrow(&self.shared_state), |s| s.inner.text().0)
    }

    pub fn right_text<'a>(&'a self) -> Ref<'a, str> {
        Ref::map(RefCell::borrow(&self.shared_state), |s| s.inner.text().1)
    }

    pub fn set_style(&mut self, style: &Rc<DualTabStyle>, font_system: &mut FontSystem) {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);

        if !Rc::ptr_eq(&shared_state.style, style) {
            shared_state.style = Rc::clone(style);
            shared_state
                .inner
                .set_style(&style.toggle_btn_style, font_system);
            self.el.notify_custom_state_change();
        }
    }

    pub fn style(&self) -> Rc<DualTabStyle> {
        Rc::clone(&RefCell::borrow(&self.shared_state).style)
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);

        if shared_state.inner.toggled() != toggled {
            shared_state.inner.set_toggled(toggled);
            self.el.notify_custom_state_change();
        }
    }

    pub fn toggled(&self) -> bool {
        RefCell::borrow(&self.shared_state).inner.toggled()
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);
        let SharedState { inner, style, .. } = &mut *shared_state;

        if disabled && inner.state() != ButtonState::Disabled {
            inner.set_state(ButtonState::Disabled, &style.toggle_btn_style);
            self.el.notify_custom_state_change();
        } else if !disabled && inner.state() == ButtonState::Disabled {
            inner.set_state(ButtonState::Idle, &style.toggle_btn_style);
            self.el.notify_custom_state_change();
        }
    }

    /// An offset that can be used mainly to correct the position of icon glyphs.
    /// This does not effect the position of the background quad.
    pub fn set_left_text_offset(&mut self, offset: Point) {
        let changed = RefCell::borrow_mut(&self.shared_state)
            .inner
            .set_left_text_offset(offset);

        if changed {
            self.el.notify_custom_state_change();
        }
    }

    /// An offset that can be used mainly to correct the position of icon glyphs.
    /// This does not effect the position of the background quad.
    pub fn set_right_text_offset(&mut self, offset: Point) {
        let changed = RefCell::borrow_mut(&self.shared_state)
            .inner
            .set_right_text_offset(offset);

        if changed {
            self.el.notify_custom_state_change();
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct DualTabGroupOption {
    pub left_text: String,
    pub right_text: String,
    pub tooltip_message: String,
    pub left_text_offset: Point,
    pub right_text_offset: Point,
}

impl DualTabGroupOption {
    pub fn new(
        left_text: impl Into<String>,
        right_text: impl Into<String>,
        tooltip_message: impl Into<String>,
        left_text_offset: Point,
        right_text_offset: Point,
    ) -> Self {
        Self {
            left_text: left_text.into(),
            right_text: right_text.into(),
            tooltip_message: tooltip_message.into(),
            left_text_offset,
            right_text_offset,
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for DualTabGroupOption {
    fn from(t: (&'a str, &'a str)) -> Self {
        Self::new(t.0, t.1, "", Point::default(), Point::default())
    }
}

impl From<(String, String)> for DualTabGroupOption {
    fn from(t: (String, String)) -> Self {
        Self::new(t.0, t.1, "", Point::default(), Point::default())
    }
}

impl<'a> From<(&'a str, &'a str, &'a str)> for DualTabGroupOption {
    fn from(t: (&'a str, &'a str, &'a str)) -> Self {
        Self::new(t.0, t.1, t.2, Point::default(), Point::default())
    }
}

impl From<(String, String, String)> for DualTabGroupOption {
    fn from(t: (String, String, String)) -> Self {
        Self::new(t.0, t.1, t.2, Point::default(), Point::default())
    }
}

pub struct DualTabGroup {
    tabs: Vec<DualTab>,
    selected_index: usize,
    bounds: Rect,
}

impl DualTabGroup {
    pub fn new<'a, A: Clone + 'static, F>(
        options: impl IntoIterator<Item = impl Into<DualTabGroupOption>>,
        selected_index: usize,
        mut on_selected: F,
        style: &Rc<DualTabStyle>,
        z_index: u16,
        on_indicator_line_placement: IndicatorLinePlacement,
        tooltip_align: Align2,
        cx: &mut WindowContext<A>,
    ) -> Self
    where
        F: FnMut(usize) -> A + 'static,
    {
        let tabs: Vec<DualTab> = options
            .into_iter()
            .enumerate()
            .map(|(i, option)| {
                let option: DualTabGroupOption = option.into();

                DualTab::builder(style)
                    .left_text(option.left_text)
                    .right_text(option.right_text)
                    .tooltip_message(option.tooltip_message, tooltip_align)
                    .on_toggled_on((on_selected)(i))
                    .toggled(i == selected_index)
                    .on_indicator_line_placement(on_indicator_line_placement)
                    .z_index(z_index)
                    .left_text_offset(option.left_text_offset)
                    .right_text_offset(option.right_text_offset)
                    .build(cx)
            })
            .collect();

        Self {
            tabs,
            selected_index,
            bounds: Rect::default(),
        }
    }

    pub fn layout(
        &mut self,
        origin: Point,
        spacing: f32,
        direction: LayoutDirection,
        stretch_to_fit: Option<f32>,
    ) {
        self.bounds.origin = origin;

        if self.tabs.is_empty() {
            self.bounds.size = Size::default();
            return;
        }

        if let LayoutDirection::Horizontal = direction {
            let mut x = origin.x;

            let max_height = stretch_to_fit.unwrap_or_else(|| {
                let mut max_height: f32 = 0.0;
                for tab in self.tabs.iter() {
                    let size = tab.desired_padded_size();
                    max_height = max_height.max(size.height);
                }
                max_height
            });

            for tab in self.tabs.iter_mut() {
                let size = tab.desired_padded_size();

                tab.el.set_rect(Rect::new(
                    Point::new(x, origin.y),
                    Size::new(size.width, max_height),
                ));

                x += size.width + spacing;
            }

            self.bounds = Rect::new(origin, Size::new(x - spacing, max_height));
        } else {
            let mut y = origin.y;

            let max_width = stretch_to_fit.unwrap_or_else(|| {
                let mut max_width: f32 = 0.0;
                for tab in self.tabs.iter() {
                    let size = tab.desired_padded_size();
                    max_width = max_width.max(size.width);
                }
                max_width
            });

            for tab in self.tabs.iter_mut() {
                let size = tab.desired_padded_size();

                tab.el.set_rect(Rect::new(
                    Point::new(origin.x, y),
                    Size::new(max_width, size.height),
                ));

                y += size.height + spacing;
            }

            self.bounds = Rect::new(origin, Size::new(max_width, y - spacing));
        }
    }

    pub fn updated_selected(&mut self, selected_index: usize) {
        let selected_index = if selected_index >= self.tabs.len() {
            0
        } else {
            selected_index
        };

        if self.selected_index == selected_index {
            return;
        }

        if let Some(prev_selected_tab) = self.tabs.get_mut(self.selected_index) {
            prev_selected_tab.set_toggled(false);
        }

        self.selected_index = selected_index;

        self.tabs[selected_index].set_toggled(true);
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        for tab in self.tabs.iter_mut() {
            tab.el.set_hidden(hidden);
        }
    }
}
