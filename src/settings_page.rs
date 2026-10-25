//! Home, radius and altitude settings, shown in place of the main view.

use crate::app::{Overhead, title_bar, title_button};
use crate::feed::{AltitudeRange, MAX_ALTITUDE_FT, MAX_RADIUS_MI};
use crate::location;
use crate::number_field::{Committed, NumberField};
use crate::setup::switch;
use crate::theme::Theme;
use crate::{ChangeLocation, CloseSettings};
use gpui::{
    AnyElement, ClickEvent, Context, Entity, FontWeight, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, div, prelude::*, px,
};

pub struct SettingsPage {
    main: Entity<Overhead>,
    radius: Entity<NumberField>,
    min_altitude: Entity<NumberField>,
    max_altitude: Entity<NumberField>,
    _subscriptions: [Subscription; 3],
}

impl SettingsPage {
    pub fn new(main: Entity<Overhead>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (radius_mi, altitude) = {
            let main = main.read(cx);
            (main.radius_mi, main.altitude)
        };
        let radius =
            cx.new(|cx| NumberField::new("radius", Some(radius_mi), 1..=MAX_RADIUS_MI, window, cx));
        let mut altitude_field = |id, value, cx: &mut Context<Self>| {
            cx.new(|cx| {
                NumberField::new(id, value, 0..=MAX_ALTITUDE_FT, window, cx)
                    .step(1000)
                    .placeholder("Any")
            })
        };
        let min_altitude = altitude_field("min-altitude", altitude.min_ft, cx);
        let max_altitude = altitude_field("max-altitude", altitude.max_ft, cx);

        Self {
            _subscriptions: [
                cx.subscribe(&radius, |this, _, Committed(radius), cx| {
                    if let Some(radius_mi) = radius {
                        this.main
                            .update(cx, |main, cx| main.set_radius(*radius_mi, cx));
                    }
                }),
                cx.subscribe(&min_altitude, |this, _, _: &Committed, cx| {
                    this.commit_altitude(cx)
                }),
                cx.subscribe(&max_altitude, |this, _, _: &Committed, cx| {
                    this.commit_altitude(cx)
                }),
            ],
            main,
            radius,
            min_altitude,
            max_altitude,
        }
    }

    fn commit_altitude(&mut self, cx: &mut Context<Self>) {
        let range = AltitudeRange {
            min_ft: self.min_altitude.read(cx).value(),
            max_ft: self.max_altitude.read(cx).value(),
        };
        let range = self.main.update(cx, |main, cx| {
            main.set_altitude(range, cx);
            main.altitude
        });
        // The bounds may have been swapped into order.
        self.min_altitude
            .update(cx, |field, cx| field.set_value(range.min_ft, cx));
        self.max_altitude
            .update(cx, |field, cx| field.set_value(range.max_ft, cx));
    }
}

impl Render for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let main = self.main.read(cx);
        let (home, approximate) = (main.home, main.approximate);
        let (hover_bg, hover_text) = (theme.accent_soft, theme.accent);

        let home_section = section(
            "Home",
            vec![
                row(
                    "Location",
                    location::format_coords(home),
                    div()
                        .id("change-home")
                        .px(px(10.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .rounded(px(7.))
                        .bg(theme.card)
                        .border_1()
                        .border_color(theme.border)
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover_bg).text_color(hover_text))
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(ChangeLocation), cx)
                        })
                        .child("Change…"),
                    &theme,
                ),
                div()
                    .id("approximate")
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.main
                            .update(cx, |main, cx| main.set_approximate(!approximate, cx));
                        cx.notify();
                    }))
                    .child(row(
                        "Share only an approximate location",
                        "Flight data services see a 1 km area, not your house.".into(),
                        switch(approximate, &theme),
                        &theme,
                    ))
                    .into_any_element(),
            ],
            None,
            &theme,
        );

        let zone_section = section(
            "Zone",
            vec![
                row(
                    "Radius",
                    format!("How far from home to watch, up to {MAX_RADIUS_MI} mi."),
                    with_unit(self.radius.clone(), "mi", &theme),
                    &theme,
                ),
                row(
                    "Lowest altitude",
                    "Any includes aircraft on the ground.".into(),
                    with_unit(self.min_altitude.clone(), "ft", &theme),
                    &theme,
                ),
                row(
                    "Highest altitude",
                    "Lower this to skip high overflights.".into(),
                    with_unit(self.max_altitude.clone(), "ft", &theme),
                    &theme,
                ),
            ],
            Some(
                "Altitudes are barometric, in feet above sea level. Aircraft that don't report an altitude always count.",
            ),
            &theme,
        );

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .child(
                title_bar().child(
                    title_button("back", "←", &theme).on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(CloseSettings), cx)
                    }),
                ),
            )
            .child(
                div()
                    .id("settings")
                    .flex_1()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .px(px(20.))
                    .pb(px(24.))
                    .child(
                        div()
                            .pt(px(4.))
                            .text_size(px(26.))
                            .font_weight(FontWeight::BOLD)
                            .child("Settings"),
                    )
                    .child(home_section)
                    .child(zone_section),
            )
    }
}

fn section(
    title: &'static str,
    rows: Vec<AnyElement>,
    note: Option<&'static str>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .px(px(2.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(px(14.))
                .bg(theme.card)
                .border_1()
                .border_color(theme.border)
                .children(rows.into_iter().enumerate().map(|(i, row)| {
                    div()
                        .when(i > 0, |d| d.border_t_1().border_color(theme.border))
                        .child(row)
                })),
        )
        .children(note.map(|note| {
            div()
                .px(px(2.))
                .text_size(px(12.))
                .text_color(theme.faint)
                .child(note)
        }))
}

fn row(
    label: &'static str,
    detail: String,
    control: impl IntoElement,
    theme: &Theme,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .px(px(14.))
        .py(px(12.))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(div().font_weight(FontWeight::MEDIUM).child(label))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted)
                        .child(detail),
                ),
        )
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

fn with_unit(field: Entity<NumberField>, unit: &'static str, theme: &Theme) -> impl IntoElement {
    div().flex().items_center().gap(px(6.)).child(field).child(
        div()
            .w(px(16.))
            .whitespace_nowrap()
            .text_color(theme.muted)
            .child(unit),
    )
}
