//! A click-to-edit number. Type digits, then press Return or click away to commit.

use crate::app::thousands;
use crate::theme::Theme;
use gpui::{
    Context, EventEmitter, FocusHandle, FontWeight, IntoElement, KeyDownEvent, ParentElement,
    Render, Styled, Subscription, Window, div, prelude::*, px,
};
use std::ops::RangeInclusive;

/// The committed value. `None` only for fields with a placeholder, when left blank.
pub struct Committed(pub Option<u32>);

/// The number being typed. `replace` is true until the first keystroke, like a selected field.
struct Draft {
    text: String,
    replace: bool,
}

pub struct NumberField {
    id: &'static str,
    value: Option<u32>,
    range: RangeInclusive<u32>,
    step: u32,
    /// Shown for no value. Without one, the field can't be left blank.
    placeholder: Option<&'static str>,
    compact: bool,
    draft: Option<Draft>,
    focus: FocusHandle,
    _blur: Subscription,
}

impl EventEmitter<Committed> for NumberField {}

impl NumberField {
    pub fn new(
        id: &'static str,
        value: Option<u32>,
        range: RangeInclusive<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        Self {
            id,
            value,
            range,
            step: 1,
            placeholder: None,
            compact: false,
            draft: None,
            _blur: cx.on_blur(&focus, window, |this, _, cx| this.commit(cx)),
            focus,
        }
    }

    /// How far the up and down arrow keys move the value.
    pub fn step(mut self, step: u32) -> Self {
        self.step = step;
        self
    }

    pub fn placeholder(mut self, placeholder: &'static str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Sized to sit inline with body text.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn value(&self) -> Option<u32> {
        self.value
    }

    pub fn set_value(&mut self, value: Option<u32>, cx: &mut Context<Self>) {
        self.value = value;
        self.draft = None;
        cx.notify();
    }

    fn edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.draft.is_none() {
            self.draft = Some(Draft {
                text: self.value.map(|v| v.to_string()).unwrap_or_default(),
                replace: true,
            });
            window.focus(&self.focus);
            cx.notify();
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (min, max) = (*self.range.start(), *self.range.end());
        let max_digits = max.to_string().len();
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let key = event.keystroke.key.as_str();
        match key {
            "enter" => window.blur(),
            "escape" => {
                self.draft = None;
                window.blur();
            }
            "backspace" if draft.replace => {
                draft.text.clear();
                draft.replace = false;
            }
            "backspace" => {
                draft.text.pop();
            }
            "up" | "down" => {
                let value = draft.text.parse().ok().or(self.value).unwrap_or(min);
                let value = if key == "up" {
                    value.saturating_add(self.step)
                } else {
                    value.saturating_sub(self.step)
                };
                draft.text = value.clamp(min, max).to_string();
                draft.replace = true;
            }
            digit if digit.len() == 1 && digit.chars().all(|c| c.is_ascii_digit()) => {
                if draft.replace {
                    draft.text.clear();
                    draft.replace = false;
                }
                if draft.text.len() < max_digits {
                    draft.text.push_str(digit);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn commit(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.draft.take() else {
            return;
        };
        cx.notify();
        let value = match draft.text.parse::<u32>() {
            Ok(value) => Some(value.clamp(*self.range.start(), *self.range.end())),
            Err(_) if draft.text.is_empty() && self.placeholder.is_some() => None,
            Err(_) => return,
        };
        if value != self.value {
            self.value = value;
            cx.emit(Committed(value));
        }
    }
}

impl Render for NumberField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let (hover_bg, hover_text) = (theme.accent_soft, theme.accent);
        let field = div()
            .id(self.id)
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .justify_center()
            .font_weight(FontWeight::SEMIBOLD)
            .map(|field| {
                if self.compact {
                    field
                        .mx(px(3.))
                        .px(px(5.))
                        .h(px(20.))
                        .min_w(px(22.))
                        .rounded(px(5.))
                } else {
                    field.px(px(10.)).h(px(28.)).min_w(px(72.)).rounded(px(7.))
                }
            })
            .on_click(cx.listener(|this, _, window, cx| this.edit(window, cx)))
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.commit(cx)));

        match &self.draft {
            None => field
                .bg(theme.card)
                .text_color(if self.value.is_some() {
                    theme.text
                } else {
                    theme.faint
                })
                .when(!self.compact, |field| {
                    field.border_1().border_color(theme.border)
                })
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).text_color(hover_text))
                .child(match self.value {
                    Some(value) => thousands(value as i64),
                    None => self.placeholder.unwrap_or_default().to_string(),
                }),
            Some(draft) => field
                .bg(theme.accent_soft)
                .text_color(theme.accent)
                .border_1()
                .border_color(theme.accent)
                .cursor_text()
                .map(|field| {
                    if draft.replace {
                        field.child(
                            div()
                                .px(px(1.))
                                .rounded(px(3.))
                                .bg(theme.accent_soft)
                                .child(draft.text.clone()),
                        )
                    } else {
                        field
                            .child(draft.text.clone())
                            .child(div().ml(px(1.)).w(px(1.5)).h(px(13.)).bg(theme.accent))
                    }
                }),
        }
    }
}
