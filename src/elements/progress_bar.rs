use std::cell::RefCell;
use std::rc::Rc;

use crate::prelude::*;

// The style of a [`ProgressBar`] element.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBarStyle {
    /// The style of the background rectangle.
    ///
    /// Set to `QuadStyle::TRANSPARENT` for no background rectangle.
    ///
    /// By default this is set to `QuadStyle::TRANSPARENT`.
    pub back_quad: QuadStyle,

    /// The style of the fill rectangle which represents the percent in the display.
    ///
    /// Set to `QuadStyle::TRANSPARENT` for no fill rectangle.
    ///
    /// By default this is set to `QuadStyle::TRANSPARENT`.
    pub fill_quad: QuadStyle,
}

impl Default for ProgressBarStyle {
    fn default() -> Self {
        Self {
            back_quad: QuadStyle::TRANSPARENT,
            fill_quad: QuadStyle::TRANSPARENT,
        }
    }
}

impl ElementStyle for ProgressBarStyle {
    const ID: &'static str = "progbar";

    fn default_dark_style() -> Self {
        Self::default()
    }

    fn default_light_style() -> Self {
        Self::default()
    }
}

#[element_builder]
#[element_builder_class]
#[element_builder_rect]
#[element_builder_hidden]
#[element_builder_disabled]
#[element_builder_tooltip]
#[derive(Default)]
pub struct ProgressBarBuilder {
    percent: f32,
}

impl ProgressBarBuilder {
    pub fn build<A: Clone + 'static>(self, cx: &mut WindowContext<'_, A>) -> ProgressBar {
        ProgressBarElement::create(self, cx)
    }

    /// Set the initial percent to be displayed by the progress bar, within the
    /// range [0.0, 100.0]. Values outside of this range will be clamped.
    pub fn percent(mut self, percent: f32) -> Self {
        let percent = clamp_percent(percent);
        self.percent = percent;
        self
    }
}

/// A simple element that displays a horizontal progress bar.
pub struct ProgressBarElement {
    shared_state: Rc<RefCell<SharedState>>,
}

impl ProgressBarElement {
    fn create<A: Clone + 'static>(
        builder: ProgressBarBuilder, 
        cx: &mut WindowContext<'_, A>
    ) -> ProgressBar {
        let ProgressBarBuilder {
            percent,
            class,
            z_index,
            rect,
            manually_hidden,
            scissor_rect,
            tooltip_data,
            disabled,
        } = builder;

        let (z_index, scissor_rect, class) = cx.builder_values(z_index, scissor_rect, class);

        let shared_state = Rc::new(RefCell::new(SharedState {
            percent: percent,
            disabled,
            tooltip_inner: TooltipInner::new(tooltip_data),
        }));

        let element_builder = ElementBuilder {
            element: Box::new(Self {
                shared_state: Rc::clone(&shared_state),
            }),
            z_index,
            rect,
            manually_hidden,
            scissor_rect,
            class,
        };

        let el = cx
            .view
            .add_element(element_builder, &mut cx.res, cx.clipboard);

        ProgressBar { el, shared_state }
    }
}

impl<A: Clone + 'static> Element<A> for ProgressBarElement {
    fn flags(&self) -> ElementFlags {
        ElementFlags::PAINTS |
        ElementFlags::LISTENS_TO_POINTER_INSIDE_BOUNDS
    }

    fn on_event(
        &mut self,
        event: ElementEvent,
        cx: &mut ElementContext<'_, A>
    ) -> EventCaptureStatus {
        let shared_state = RefCell::borrow_mut(&self.shared_state);

        shared_state
            .tooltip_inner
            .handle_event(&event, shared_state.disabled, cx);

        if let ElementEvent::CustomStateChanged = event {
            cx.request_repaint();
        }

        EventCaptureStatus::NotCaptured
    }

    fn render_primitives(&mut self, cx: RenderContext<'_>, primitives: &mut PrimitiveGroup) {
        let shared_state = RefCell::borrow(&self.shared_state);
        let style: &ProgressBarStyle = cx.res.style_system.get(cx.class);

        let bounds = Rect::from_size(cx.bounds_size);
        let filled_width = bounds.width() * shared_state.percent / 100.0;
        
        if !style.back_quad.is_transparent() {
            let back_primitive = style.back_quad.create_primitive(bounds);

            primitives.add(back_primitive);
        }

        if !style.fill_quad.is_transparent() {
            let fill_origin = bounds.min();
            //let max_point = Point::new(bounds.min_x() + filled_width, bounds.max_y());
            let fill_size = Size::new(filled_width, bounds.height());
            let fill_rect = Rect::new(fill_origin, fill_size);
            let fill_primitive = style.fill_quad.create_primitive(fill_rect);

            primitives.set_z_index(1);
            primitives.add(fill_primitive);
        }
    }
}

struct SharedState {
    percent: f32,
    disabled: bool,
    tooltip_inner: TooltipInner,
}

/// A handle to a [`ProgressBarElement`], a simple element that displays a horizontal progress bar.
#[element_handle]
#[element_handle_class]
#[element_handle_set_rect]
#[element_handle_set_tooltip]
pub struct ProgressBar {
    shared_state: Rc<RefCell<SharedState>>,
}

impl ProgressBar {
    pub fn builder() -> ProgressBarBuilder {
        ProgressBarBuilder::default()
    }

    /// Set the percent displayed by the progress bar, within the range
    /// [0.0, 100.0]. Values outside of this range will be clamped.
    ///
    /// Returns `true` if the percent has changed.
    ///
    /// This will *NOT* trigger an element update unless the value has changed,
    /// so this method is relatively cheap to call frequently.
    pub fn set_percent(&mut self, percent: f32) -> bool {
        // clamp percent to range [0.0, 100.0]
        let percent = clamp_percent(percent);

        let mut shared_state = RefCell::borrow_mut(&self.shared_state);

        if shared_state.percent == percent {
            false
        } else {
            shared_state.percent = percent;
            self.el.notify_custom_state_change();
            true
        }
    }

    /// Returns the percent displayed by the progress bar, which is clamped
    /// to the range [0.0, 100.0].
    pub fn percent(&self) -> f32 {
        RefCell::borrow(&self.shared_state).percent
    }

    /// Set the disabled state of this element.
    ///
    /// Returns `true` if the disabled state has changed.
    ///
    /// This will *NOT* trigger an element update unless the state has changed,
    /// so this method is relatively inexpensive to call.
    pub fn set_disabled(&mut self, disabled: bool) -> bool {
        let mut shared_state = RefCell::borrow_mut(&self.shared_state);

        if shared_state.disabled != disabled {
            shared_state.disabled = disabled;
            self.el.notify_custom_state_change();
            true
        } else {
            false
        }
    }

    pub fn disabled(&self) -> bool {
        let shared_state = RefCell::borrow(&self.shared_state);
        shared_state.disabled
    }

}

fn clamp_percent(percent: f32) -> f32 {
    if percent > 100.0 {
        100.0
    } else if percent < 0.0 {
        0.0
    } else {
        percent
    }
}