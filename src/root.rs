use crate::app::Overhead;
use crate::location::Coords;
use crate::settings;
use crate::setup::{Setup, SetupEvent};
use gpui::{
    Animation, AnimationExt, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, ease_out_quint, prelude::*,
};
use std::time::Duration;

/// Shows setup until home is known, then the main view.
pub struct Root {
    main: Option<Entity<Overhead>>,
    setup: Option<(Entity<Setup>, Subscription)>,
    /// Bumped on every switch so the main view fades in again.
    shown: usize,
    _appearance: Subscription,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::blur::fix(window);
        let mut root = Self {
            main: None,
            setup: None,
            shown: 0,
            _appearance: cx.observe_window_appearance(window, |_, _, cx| cx.notify()),
        };
        match settings::load_home() {
            Some(home) => root.show_main(home, window, cx),
            None => root.show_setup(true, window, cx),
        }
        root
    }

    fn show_main(&mut self, home: Coords, window: &mut Window, cx: &mut Context<Self>) {
        self.main = Some(cx.new(|cx| Overhead::new(home, window, cx)));
        self.setup = None;
        self.shown += 1;
        cx.notify();
    }

    fn show_setup(&mut self, first_run: bool, window: &mut Window, cx: &mut Context<Self>) {
        let setup = cx.new(|cx| Setup::new(first_run, window, cx));
        let subscription =
            cx.subscribe_in(&setup, window, |this, _, event, window, cx| match event {
                SetupEvent::Done(home) => this.show_main(*home, window, cx),
                SetupEvent::Cancel => {
                    this.setup = None;
                    this.shown += 1;
                    cx.notify();
                }
            });
        self.setup = Some((setup, subscription));
        cx.notify();
    }

    pub fn change_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.setup.is_none() {
            self.show_setup(false, window, cx);
        }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full();
        match (&self.setup, &self.main) {
            (Some((setup, _)), _) => root.child(setup.clone()),
            (None, Some(main)) => root.child(div().size_full().child(main.clone()).with_animation(
                ("main", self.shown),
                Animation::new(Duration::from_millis(450)).with_easing(ease_out_quint()),
                |main, t| main.opacity(t),
            )),
            (None, None) => root,
        }
    }
}
