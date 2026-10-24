//! First launch: where home is, then notifications. Also reused to change home later.

use crate::location::{self, Coords, Place};
use crate::sound::{self, Sound};
use crate::theme::Theme;
use crate::{feed, notify, settings};
use gpui::{
    Animation, AnimationElement, AnimationExt, ClickEvent, Context, ElementId, EventEmitter,
    FocusHandle, FontWeight, Hsla, IntoElement, KeyDownEvent, ParentElement, Render, Styled,
    Subscription, Task, Window, div, ease_out_quint, prelude::*, px,
};
use std::time::Duration;

const RISE: Duration = Duration::from_millis(520);
const STAGGER: Duration = Duration::from_millis(60);
/// Characters shown in the search field before older text scrolls off.
const VISIBLE_CHARS: usize = 38;

pub enum SetupEvent {
    Done(Coords),
    Cancel,
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Location,
    Notifications,
}

enum Lookup {
    Idle,
    Locating,
    Searching,
    Found(Place),
    Failed(String),
}

pub struct Setup {
    first_run: bool,
    page: Page,
    query: String,
    lookup: Lookup,
    /// Keys the result card's animation so each new result plays it again.
    lookups: usize,
    home: Option<Coords>,
    focus: FocusHandle,
    task: Option<Task<()>>,
    _focus_changes: [Subscription; 2],
}

impl EventEmitter<SetupEvent> for Setup {}

impl Setup {
    pub fn new(first_run: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        Self {
            first_run,
            page: Page::Location,
            query: String::new(),
            lookup: Lookup::Idle,
            lookups: 0,
            home: None,
            _focus_changes: [
                cx.on_focus(&focus, window, |_, _, cx| cx.notify()),
                cx.on_blur(&focus, window, |_, _, cx| cx.notify()),
            ],
            focus,
            task: None,
        }
    }

    fn use_current_location(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            let reply = location::locate();
            self.lookup = Lookup::Locating;
            self.resolve(cx, move || {
                let coords = reply
                    .recv_timeout(Duration::from_secs(30))
                    .unwrap_or_else(|_| {
                        Err("Couldn't find your location. Try typing your address instead.".into())
                    })?;
                let label = location::reverse_geocode(&feed::agent(), coords)
                    .unwrap_or_else(|| "Your current location".into());
                Ok(Place { coords, label })
            });
        }
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.query.trim().to_string();
        if query.is_empty() {
            return;
        }
        if let Some(coords) = location::parse_coords(&query) {
            self.lookup = Lookup::Searching;
            self.resolve(cx, move || {
                let label = location::reverse_geocode(&feed::agent(), coords)
                    .unwrap_or_else(|| "Coordinates".into());
                Ok(Place { coords, label })
            });
        } else {
            self.lookup = Lookup::Searching;
            self.resolve(cx, move || location::geocode(&feed::agent(), &query));
        }
    }

    fn resolve(
        &mut self,
        cx: &mut Context<Self>,
        find: impl FnOnce() -> Result<Place, String> + Send + 'static,
    ) {
        cx.notify();
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { find() }).await;
            this.update(cx, |this, cx| {
                this.lookups += 1;
                this.lookup = match result {
                    Ok(place) => {
                        sound::play(Sound::Found);
                        Lookup::Found(place)
                    }
                    Err(err) => Lookup::Failed(err),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let command = keystroke.modifiers.platform;
        match keystroke.key.as_str() {
            "enter" => {
                self.search(cx);
                return cx.stop_propagation();
            }
            "escape" => window.blur(),
            "backspace" if command => self.query.clear(),
            "backspace" if keystroke.modifiers.alt => {
                let trimmed = self.query.trim_end().len();
                let word_start = self.query[..trimmed].rfind(' ').map_or(0, |i| i + 1);
                self.query.truncate(word_start);
            }
            "backspace" => {
                self.query.pop();
            }
            "v" if command => {
                let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                    return;
                };
                self.query.push_str(&text.replace(['\n', '\r', '\t'], " "));
            }
            _ if command || keystroke.modifiers.control => return,
            _ => match &keystroke.key_char {
                Some(text) if !text.chars().any(char::is_control) => self.query.push_str(text),
                _ => return,
            },
        }
        if !matches!(self.lookup, Lookup::Idle) {
            self.lookup = Lookup::Idle;
            self.task = None;
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn save_location(&mut self, cx: &mut Context<Self>) {
        let Lookup::Found(place) = &self.lookup else {
            return;
        };
        let home = place.coords;
        settings::save_home(home);
        self.home = Some(home);
        if self.first_run {
            sound::play(Sound::Next);
            self.page = Page::Notifications;
            cx.notify();
        } else {
            sound::play(Sound::Done);
            cx.emit(SetupEvent::Done(home));
        }
    }

    fn finish(&mut self, allow_notifications: bool, cx: &mut Context<Self>) {
        if allow_notifications {
            notify::request_permission();
        }
        if let Some(home) = self.home {
            sound::play(Sound::Done);
            cx.emit(SetupEvent::Done(home));
        }
    }

    fn location_page(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let found = matches!(self.lookup, Lookup::Found(_));
        let busy = matches!(self.lookup, Lookup::Locating);

        let footer =
            if self.first_run {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .child(
                        primary_button("continue", "Continue", found, theme).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| this.save_location(cx)),
                        ),
                    )
                    .child(steps(self.page, theme))
            } else {
                div()
                    .flex()
                    .gap(px(10.))
                    .child(secondary_button("cancel", "Cancel", theme).on_click(
                        cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SetupEvent::Cancel)),
                    ))
                    .child(primary_button("save", "Save", found, theme).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.save_location(cx)),
                    ))
            };

        page(
            Page::Location,
            radar(theme),
            if self.first_run {
                "Where's home?"
            } else {
                "Move home"
            },
            "Overhead watches the sky around one spot and tells you when military aircraft fly over it.",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(rise(
                    secondary_button(
                        "locate",
                        if busy {
                            "Finding you…"
                        } else {
                            "Use My Current Location"
                        },
                        theme,
                    )
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.use_current_location(cx)),
                    ),
                    "locate",
                    3,
                ))
                .child(rise(divider(theme), "or", 4))
                .child(rise(self.search_field(theme, window, cx), "field", 5))
                .child(div().h(px(66.)).child(self.lookup_status(theme))),
            footer,
        )
    }

    fn search_field(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let focused = self.focus.is_focused(window);
        let text = if self.query.chars().count() > VISIBLE_CHARS {
            let tail: String = self.query.chars().rev().take(VISIBLE_CHARS - 1).collect();
            format!("…{}", tail.chars().rev().collect::<String>())
        } else {
            self.query.clone()
        };

        div()
            .id("query")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                window.focus(&this.focus);
                cx.notify();
            }))
            .h(px(44.))
            .px(px(14.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(12.))
            .bg(theme.card)
            .border_1()
            .border_color(if focused { theme.accent } else { theme.border })
            .cursor_text()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(self.query.is_empty(), |field| {
                        field
                            .when(focused, |field| field.child(caret(theme)))
                            .child(
                                div()
                                    .text_color(theme.faint)
                                    .child("Address, or latitude, longitude"),
                            )
                    })
                    .when(!self.query.is_empty(), |field| {
                        field
                            .child(text)
                            .when(focused, |field| field.child(caret(theme)))
                    }),
            )
            .when(!self.query.is_empty(), |field| {
                field.child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.))
                        .text_color(theme.faint)
                        .child("return ↵"),
                )
            })
    }

    fn lookup_status(&self, theme: &Theme) -> impl IntoElement {
        let note = |text: String, color: Hsla| {
            div()
                .pt(px(2.))
                .px(px(2.))
                .text_size(px(12.))
                .text_color(color)
                .child(text)
        };
        match &self.lookup {
            Lookup::Idle => div().into_any_element(),
            Lookup::Locating => pulse(
                note("Waiting for your location…".into(), theme.muted),
                "locating",
            )
            .into_any_element(),
            Lookup::Searching => {
                pulse(note("Searching…".into(), theme.muted), "searching").into_any_element()
            }
            Lookup::Failed(err) => {
                rise(note(err.clone(), theme.error), ("failed", self.lookups), 0).into_any_element()
            }
            Lookup::Found(place) => {
                rise(found_card(place, theme), ("found", self.lookups), 0).into_any_element()
            }
        }
    }

    fn notifications_page(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        page(
            Page::Notifications,
            sample_notification(theme),
            "Know when one flies over",
            "Get a notification the moment a military aircraft enters your zone, even while Overhead is in the background.",
            div(),
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    primary_button("allow", "Turn On Notifications", true, theme)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish(true, cx))),
                )
                .child(
                    text_button("later", "Not Now", theme).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.finish(false, cx)),
                    ),
                )
                .child(div().mt(px(8.)).child(steps(self.page, theme))),
        )
    }
}

impl Render for Setup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let content = match self.page {
            Page::Location => self.location_page(&theme, window, cx).into_any_element(),
            Page::Notifications => self.notifications_page(&theme, cx).into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .child(div().h(px(44.)).flex_shrink_0())
            .child(content)
    }
}

/// The shared layout: art, title and text centered above the controls and footer.
fn page(
    page: Page,
    art: impl IntoElement,
    title: &'static str,
    body: &'static str,
    controls: impl IntoElement,
    footer: impl IntoElement,
) -> impl IntoElement {
    let key = page as usize;
    div()
        .flex_1()
        .flex()
        .flex_col()
        .px(px(32.))
        .pb(px(28.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .child(rise(div().child(art), ("art", key), 0))
                .child(rise(
                    div()
                        .mt(px(28.))
                        .text_size(px(26.))
                        .font_weight(FontWeight::BOLD)
                        .child(title),
                    ("title", key),
                    1,
                ))
                .child(rise(
                    div()
                        .mt(px(8.))
                        .max_w(px(320.))
                        .text_center()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .opacity(0.62)
                        .child(body),
                    ("body", key),
                    2,
                )),
        )
        .child(controls)
        .child(rise(div().mt(px(20.)).child(footer), ("footer", key), 6))
}

/// Fades an element in while it rises into place, `step` beats after the page appears.
fn rise<E: Styled + IntoElement + 'static>(
    element: E,
    id: impl Into<ElementId>,
    step: u32,
) -> AnimationElement<E> {
    let delay = (STAGGER * step).as_secs_f32();
    let total = delay + RISE.as_secs_f32();
    let ease = ease_out_quint();
    element.with_animation(
        id,
        Animation::new(Duration::from_secs_f32(total)),
        move |element, t| {
            let local = ease(((t * total - delay) / RISE.as_secs_f32()).clamp(0., 1.));
            element
                .relative()
                .top(px((1. - local) * 14.))
                .opacity(local)
        },
    )
}

fn pulse<E: Styled + IntoElement + 'static>(element: E, id: &'static str) -> AnimationElement<E> {
    element.with_animation(
        id,
        Animation::new(Duration::from_millis(1400)).repeat(),
        |element, t| element.opacity(0.45 + 0.55 * (t * std::f32::consts::TAU).cos().abs()),
    )
}

fn caret(theme: &Theme) -> impl IntoElement {
    div()
        .w(px(1.5))
        .h(px(16.))
        .mr(px(1.))
        .bg(theme.accent)
        .with_animation(
            "caret",
            Animation::new(Duration::from_millis(1000)).repeat(),
            |caret, t| caret.opacity(if t < 0.55 { 1. } else { 0. }),
        )
}

/// Rings around home, with a ping sweeping outward.
fn radar(theme: &Theme) -> impl IntoElement {
    const SIZE: f32 = 132.;
    let ring = |diameter: f32| {
        div()
            .absolute()
            .top(px((SIZE - diameter) / 2.))
            .left(px((SIZE - diameter) / 2.))
            .size(px(diameter))
            .rounded_full()
            .border_1()
    };
    let accent = theme.accent;

    div()
        .relative()
        .size(px(SIZE))
        .child(ring(SIZE).border_color(theme.border))
        .child(ring(SIZE * 0.66).border_color(theme.border))
        .child(ring(SIZE * 0.33).border_color(theme.border))
        .child(
            ring(12.)
                .bg(theme.accent_soft)
                .border_color(accent)
                .with_animation(
                    "ping",
                    Animation::new(Duration::from_millis(2600))
                        .repeat()
                        .with_easing(ease_out_quint()),
                    move |ring, t| {
                        let diameter = 12. + t * (SIZE - 12.);
                        ring.size(px(diameter))
                            .top(px((SIZE - diameter) / 2.))
                            .left(px((SIZE - diameter) / 2.))
                            .bg(accent.opacity(0.10 * (1. - t)))
                            .border_color(accent.opacity(1. - t))
                    },
                ),
        )
        .child(
            ring(12.)
                .border_0()
                .bg(accent)
                .shadow(vec![gpui::BoxShadow {
                    color: accent.opacity(0.5),
                    offset: gpui::point(px(0.), px(0.)),
                    blur_radius: px(10.),
                    spread_radius: px(0.),
                }]),
        )
}

fn sample_notification(theme: &Theme) -> impl IntoElement {
    let icon = div()
        .flex_shrink_0()
        .size(px(38.))
        .rounded(px(9.))
        .bg(gpui::hsla(225. / 360., 0.14, 0.12, 1.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(25.))
                .rounded_full()
                .border_1()
                .border_color(gpui::hsla(0., 0., 1., 0.22))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(4.))
                        .rounded_full()
                        .bg(gpui::hsla(0., 0., 1., 0.85)),
                ),
        );

    div()
        .w(px(340.))
        .flex()
        .items_center()
        .gap(px(12.))
        .p(px(12.))
        .rounded(px(18.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .child(icon)
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child("C-17A Globemaster III overhead"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.faint)
                                .child("now"),
                        ),
                )
                .child(
                    div()
                        .text_color(theme.muted)
                        .child("RCH871 · 2.4 mi NE · 12,000 ft"),
                ),
        )
}

fn found_card(place: &Place, theme: &Theme) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .gap(px(10.))
        .p(px(12.))
        .rounded(px(12.))
        .bg(theme.accent_soft)
        .child(
            div()
                .flex_shrink_0()
                .mt(px(1.))
                .size(px(16.))
                .rounded_full()
                .bg(theme.accent)
                .text_color(theme.on_accent)
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .flex()
                .items_center()
                .justify_center()
                .child("✓"),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(place.label.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child(location::format_coords(place.coords)),
                ),
        )
}

fn divider(theme: &Theme) -> gpui::Div {
    let line = || div().flex_1().h(px(1.)).bg(theme.border);
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .text_size(px(11.))
        .text_color(theme.faint)
        .child(line())
        .child("or")
        .child(line())
}

/// Two dots, the current page's stretched into a pill.
fn steps(current: Page, theme: &Theme) -> impl IntoElement {
    let dot = |page: Page| {
        let active = page == current;
        let accent = theme.accent;
        div()
            .h(px(6.))
            .w(px(6.))
            .rounded_full()
            .bg(if active {
                accent
            } else {
                theme.faint.opacity(0.5)
            })
            .with_animation(
                ("step", page as usize * 2 + active as usize),
                Animation::new(Duration::from_millis(420)).with_easing(ease_out_quint()),
                move |dot, t| if active { dot.w(px(6. + 12. * t)) } else { dot },
            )
    };
    div()
        .flex()
        .justify_center()
        .gap(px(6.))
        .child(dot(Page::Location))
        .child(dot(Page::Notifications))
}

fn primary_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    button(id, label)
        .flex_1()
        .bg(theme.accent)
        .text_color(theme.on_accent)
        .font_weight(FontWeight::SEMIBOLD)
        .when(enabled, |b| b.cursor_pointer().hover(|s| s.opacity(0.88)))
        .when(!enabled, |b| b.opacity(0.35))
}

fn secondary_button(
    id: &'static str,
    label: &'static str,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let hover = theme.accent_soft;
    button(id, label)
        .flex_1()
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
}

fn text_button(id: &'static str, label: &'static str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    let hover = theme.text;
    button(id, label)
        .h(px(32.))
        .text_color(theme.muted)
        .cursor_pointer()
        .hover(move |s| s.text_color(hover))
}

fn button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(44.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(14.))
        .child(label)
}
