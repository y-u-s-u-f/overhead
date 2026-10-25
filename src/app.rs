use crate::feed::{
    self, Aircraft, Altitude, AltitudeRange, Contact, DEFAULT_RADIUS_MI, MAX_RADIUS_MI,
};
use crate::location::Coords;
use crate::number_field::{Committed, NumberField};
use crate::theme::Theme;
use crate::tracker::{Sighting, Tracker};
use crate::{ChangeLocation, OpenSettings};
use crate::{notify, settings};
use chrono::{DateTime, Local};
use gpui::{
    Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Task, Window, div, prelude::*, px,
};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(10);
/// A static font for the big count. GPUI clips heavy weights of the variable system font.
const COUNT_FONT: &str = "Helvetica Neue";

enum Status {
    Connecting,
    Live { source: &'static str },
    Offline,
}

pub struct Overhead {
    /// Read by the settings page. Change these through the setters.
    pub home: Coords,
    pub approximate: bool,
    pub radius_mi: u32,
    pub altitude: AltitudeRange,
    tracker: Tracker,
    status: Status,
    radius_field: Entity<NumberField>,
    poll: Task<()>,
    _radius_committed: Subscription,
}

impl Overhead {
    pub fn new(home: Coords, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let radius_mi = settings::load_radius()
            .filter(|r| (1..=MAX_RADIUS_MI).contains(r))
            .unwrap_or(DEFAULT_RADIUS_MI);
        let radius_field = cx.new(|cx| {
            NumberField::new("radius", Some(radius_mi), 1..=MAX_RADIUS_MI, window, cx).compact()
        });
        Self {
            home,
            approximate: settings::load_approximate(),
            radius_mi,
            altitude: settings::load_altitude().sorted(),
            tracker: Tracker::default(),
            status: Status::Connecting,
            _radius_committed: cx.subscribe(&radius_field, |this, _, Committed(radius), cx| {
                if let Some(radius_mi) = radius {
                    this.set_radius(*radius_mi, cx);
                }
            }),
            radius_field,
            poll: Self::poll(cx),
        }
    }

    pub fn set_radius(&mut self, radius_mi: u32, cx: &mut Context<Self>) {
        let radius_mi = radius_mi.clamp(1, MAX_RADIUS_MI);
        if radius_mi == self.radius_mi {
            return;
        }
        self.radius_mi = radius_mi;
        settings::save_radius(radius_mi);
        self.radius_field
            .update(cx, |field, cx| field.set_value(Some(radius_mi), cx));
        self.tracker.retain(|c| c.distance_mi <= radius_mi as f64);
        self.restart_poll(cx);
    }

    pub fn set_altitude(&mut self, altitude: AltitudeRange, cx: &mut Context<Self>) {
        let altitude = altitude.sorted();
        if altitude == self.altitude {
            return;
        }
        self.altitude = altitude;
        settings::save_altitude(altitude);
        self.tracker
            .retain(|c| altitude.contains(c.aircraft.alt_baro.as_ref()));
        self.restart_poll(cx);
    }

    pub fn set_approximate(&mut self, approximate: bool, cx: &mut Context<Self>) {
        self.approximate = approximate;
        settings::save_approximate(approximate);
        self.restart_poll(cx);
    }

    /// Polls again right away so a changed zone shows up without waiting.
    fn restart_poll(&mut self, cx: &mut Context<Self>) {
        self.poll = Self::poll(cx);
        cx.notify();
    }

    fn poll(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let agent = feed::agent();
            loop {
                let Ok((home, radius_mi, approximate)) =
                    this.read_with(cx, |this, _| (this.home, this.radius_mi, this.approximate))
                else {
                    break;
                };
                let request = agent.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { feed::fetch(&request, home, radius_mi, approximate) })
                    .await;
                if this.update(cx, |this, cx| this.apply(result, cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(POLL_INTERVAL).await;
            }
        })
    }

    fn apply(
        &mut self,
        result: Result<(&'static str, Vec<Contact>), String>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok((source, mut contacts)) => {
                // A request can finish after the radius changed.
                contacts.retain(|c| {
                    c.distance_mi <= self.radius_mi as f64
                        && self.altitude.contains(c.aircraft.alt_baro.as_ref())
                });
                for contact in self.tracker.update(contacts, Local::now()) {
                    notify_arrival(&contact);
                }
                self.status = Status::Live { source };
            }
            Err(err) => {
                eprintln!("feed unavailable: {err}");
                self.status = Status::Offline;
            }
        }
        cx.notify();
    }

    fn title_bar(&self, theme: &Theme) -> impl IntoElement {
        let (dot, label, detail) = match self.status {
            Status::Connecting => (theme.faint, "Connecting…", None),
            Status::Live { source } => (theme.live, "Live", Some(source)),
            Status::Offline => (theme.error, "Offline", Some("retrying")),
        };

        title_bar()
            .justify_between()
            .child(
                title_button("settings", "⚙\u{FE0E}", theme)
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenSettings), cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(10.))
                    .py(px(4.))
                    .rounded_full()
                    .bg(theme.card)
                    .text_size(px(12.))
                    .child(div().size(px(7.)).rounded_full().bg(dot))
                    .child(div().font_weight(FontWeight::MEDIUM).child(label))
                    .children(detail.map(|d| div().text_color(theme.faint).child(d))),
            )
    }

    fn hero(&self, theme: &Theme) -> impl IntoElement {
        let count = self.tracker.current.len();
        div()
            .flex()
            .flex_col()
            .pt(px(4.))
            .child(
                div()
                    .font_family(COUNT_FONT)
                    .text_size(px(64.))
                    .line_height(px(70.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(if count > 0 { theme.accent } else { theme.text })
                    .child(count.to_string()),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Military aircraft in your zone"),
            )
            .child(
                div()
                    .mt(px(2.))
                    .flex()
                    .items_center()
                    .text_color(theme.muted)
                    .child("Within")
                    .child(self.radius_field.clone())
                    .child("mi of")
                    .child(home_link(theme)),
            )
            .children(
                altitude_summary(self.altitude)
                    .map(|summary| div().mt(px(2.)).text_color(theme.faint).child(summary)),
            )
    }

    fn current_section(&self, theme: &Theme) -> impl IntoElement {
        if self.tracker.current.is_empty() {
            return div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .p(px(16.))
                .rounded(px(14.))
                .bg(theme.card)
                .border_1()
                .border_color(theme.border)
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Clear skies"))
                .child(
                    div()
                        .text_color(theme.muted)
                        .child("You'll get a notification the moment one flies in."),
                );
        }

        div().flex().flex_col().gap(px(10.)).children(
            self.tracker
                .current
                .iter()
                .map(|sighting| current_card(sighting, theme)),
        )
    }

    fn recent_section(&self, theme: &Theme) -> impl IntoElement {
        let now = Local::now();
        let recent = &self.tracker.recent;

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Recently overhead"),
                    )
                    .when(!recent.is_empty(), |d| {
                        d.child(
                            div()
                                .text_color(theme.faint)
                                .child(recent.len().to_string()),
                        )
                    }),
            )
            .when(recent.is_empty(), |d| {
                d.child(
                    div()
                        .text_color(theme.faint)
                        .child("Aircraft that pass through will show up here."),
                )
            })
            .when(!recent.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .rounded(px(14.))
                        .bg(theme.card)
                        .border_1()
                        .border_color(theme.border)
                        .overflow_hidden()
                        .children(
                            recent
                                .iter()
                                .enumerate()
                                .map(|(i, sighting)| recent_row(sighting, i > 0, now, theme)),
                        ),
                )
            })
    }
}

impl Render for Overhead {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .child(self.title_bar(&theme))
            .child(
                div()
                    .id("content")
                    .flex_1()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(28.))
                    .px(px(20.))
                    .pb(px(24.))
                    .child(self.hero(&theme))
                    .child(self.current_section(&theme))
                    .child(self.recent_section(&theme)),
            )
    }
}

fn current_card(sighting: &Sighting, theme: &Theme) -> impl IntoElement {
    let contact = &sighting.contact;
    let aircraft = &contact.aircraft;

    div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .p(px(16.))
        .rounded(px(14.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .flex_1()
                        .min_w(px(0.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(aircraft.name()),
                                )
                                .children(aircraft.type_code.clone().map(|code| {
                                    div()
                                        .px(px(6.))
                                        .py(px(1.))
                                        .rounded(px(5.))
                                        .bg(theme.accent_soft)
                                        .text_color(theme.accent)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(code)
                                })),
                        )
                        .children(
                            aircraft
                                .description
                                .clone()
                                .map(|d| div().text_color(theme.muted).truncate().child(d)),
                        )
                        .children(
                            aircraft
                                .operator
                                .clone()
                                .map(|o| div().text_color(theme.faint).truncate().child(o)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_end()
                        .flex_shrink_0()
                        .child(
                            div()
                                .text_size(px(17.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("{:.1} mi", contact.distance_mi)),
                        )
                        .child(
                            div()
                                .text_color(theme.muted)
                                .child(format!("{} of you", contact.direction)),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .gap(px(20.))
                        .child(stat("Altitude", altitude(aircraft), theme))
                        .child(stat("Speed", speed(aircraft), theme))
                        .children(
                            aircraft
                                .registration
                                .clone()
                                .map(|reg| stat("Reg", reg, theme)),
                        ),
                )
                .child(flightradar_button(aircraft, theme)),
        )
}

fn recent_row(
    sighting: &Sighting,
    divider: bool,
    now: DateTime<Local>,
    theme: &Theme,
) -> impl IntoElement {
    let aircraft = &sighting.contact.aircraft;

    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .px(px(14.))
        .py(px(10.))
        .when(divider, |d| d.border_t_1().border_color(theme.border))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(6.))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(aircraft.name()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.faint)
                                .child(format!("closest {:.1} mi", sighting.closest_mi)),
                        ),
                )
                .children(aircraft.model().map(|model| {
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted)
                        .truncate()
                        .child(model.to_string())
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .flex_shrink_0()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(clock_time(sighting.last_seen, now)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.faint)
                        .child(time_ago(sighting.last_seen, now)),
                ),
        )
        .child(icon_button(aircraft, theme))
}

fn stat(label: &'static str, value: String, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.faint)
                .child(label),
        )
        .child(div().font_weight(FontWeight::MEDIUM).child(value))
}

fn flightradar_button(aircraft: &Aircraft, theme: &Theme) -> impl IntoElement {
    let url = aircraft.flightradar_url();
    let (hover_bg, hover_text) = (theme.accent_soft, theme.accent);

    div()
        .id(SharedString::from(format!("fr24-{}", aircraft.hex)))
        .flex_shrink_0()
        .px(px(10.))
        .py(px(5.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.card)
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(hover_text))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .child("Flightradar24 ↗")
}

fn icon_button(aircraft: &Aircraft, theme: &Theme) -> impl IntoElement {
    let url = aircraft.flightradar_url();
    let (hover_bg, hover_text) = (theme.accent_soft, theme.accent);

    div()
        .id(SharedString::from(format!("fr24-recent-{}", aircraft.hex)))
        .flex_shrink_0()
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .text_color(theme.muted)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(hover_text))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .child("↗")
}

/// The strip under the transparent titlebar, inset past the traffic lights.
pub fn title_bar() -> gpui::Div {
    div()
        .h(px(44.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .pl(px(84.))
        .pr(px(16.))
}

pub fn title_button(
    id: &'static str,
    glyph: &'static str,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let (hover_bg, hover_text) = (theme.accent_soft, theme.accent);
    div()
        .id(id)
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .text_size(px(16.))
        .text_color(theme.muted)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(hover_text))
        .child(glyph)
}

fn altitude_summary(altitude: AltitudeRange) -> Option<String> {
    let ft = |ft: u32| format!("{} ft", thousands(ft as i64));
    match (altitude.min_ft, altitude.max_ft) {
        (None, None) => None,
        (Some(min), None) => Some(format!("Above {}", ft(min))),
        (None, Some(max)) => Some(format!("Below {}", ft(max))),
        (Some(min), Some(max)) => {
            Some(format!("Between {} and {}", thousands(min as i64), ft(max)))
        }
    }
}

fn home_link(theme: &Theme) -> impl IntoElement {
    let hover_text = theme.accent;
    div()
        .id("home")
        .ml(px(4.))
        .cursor_pointer()
        .hover(move |s| s.text_color(hover_text))
        .on_click(|_, window, cx| window.dispatch_action(Box::new(ChangeLocation), cx))
        .child("home")
}

fn notify_arrival(contact: &Contact) {
    let aircraft = &contact.aircraft;
    let title = format!(
        "{} overhead",
        aircraft.model().unwrap_or("Military aircraft")
    );
    let body = format!(
        "{} · {:.1} mi {} · {}",
        aircraft.name(),
        contact.distance_mi,
        contact.direction,
        altitude(aircraft)
    );
    notify::send(&aircraft.hex, &title, &body);
}

fn altitude(aircraft: &Aircraft) -> String {
    match &aircraft.alt_baro {
        Some(Altitude::Feet(ft)) => format!("{} ft", thousands(*ft as i64)),
        Some(Altitude::Ground(_)) => "On ground".into(),
        None => "—".into(),
    }
}

fn speed(aircraft: &Aircraft) -> String {
    aircraft
        .ground_speed
        .map_or("—".into(), |kt| format!("{kt:.0} kt"))
}

pub fn thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn clock_time(time: DateTime<Local>, now: DateTime<Local>) -> String {
    if time.date_naive() == now.date_naive() {
        time.format("%-I:%M %p").to_string()
    } else {
        time.format("%b %-d, %-I:%M %p").to_string()
    }
}

fn time_ago(time: DateTime<Local>, now: DateTime<Local>) -> String {
    let minutes = (now - time).num_minutes();
    match minutes {
        ..1 => "just now".into(),
        1..60 => format!("{minutes} min ago"),
        60..1440 => format!("{} hr ago", minutes / 60),
        _ => format!("{} days ago", minutes / 1440),
    }
}

#[cfg(test)]
mod tests {
    use super::thousands;

    #[test]
    fn formats_thousands() {
        assert_eq!(thousands(825), "825");
        assert_eq!(thousands(3500), "3,500");
        assert_eq!(thousands(27000), "27,000");
        assert_eq!(thousands(-1200), "-1,200");
    }
}
