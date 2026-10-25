use crate::app::Overhead;
use crate::location::Coords;
use crate::settings;
use crate::settings_page::SettingsPage;
use crate::setup::{Setup, SetupEvent};
use gpui::{
    Animation, AnimationExt, AnyView, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, ease_out_quint, prelude::*,
};
use std::time::Duration;

/// Shows setup until home is known, then the main view. Settings and setup cover it when open.
pub struct Root {
    main: Option<Entity<Overhead>>,
    settings: Option<Entity<SettingsPage>>,
    setup: Option<(Entity<Setup>, Subscription)>,
    /// Bumped on every switch so the view underneath fades in again.
    shown: usize,
    _appearance: Subscription,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::blur::fix(window);
        let mut root = Self {
            main: None,
            settings: None,
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
        let main = cx.new(|cx| Overhead::new(home, window, cx));
        if self.settings.is_some() {
            self.settings = Some(cx.new(|cx| SettingsPage::new(main.clone(), window, cx)));
        }
        self.main = Some(main);
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

    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(main) = self.main.clone() else {
            return;
        };
        if self.settings.is_none() && self.setup.is_none() {
            self.settings = Some(cx.new(|cx| SettingsPage::new(main, window, cx)));
            self.shown += 1;
            cx.notify();
        }
    }

    pub fn close_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings.is_some() && self.setup.is_none() {
            self.settings = None;
            self.shown += 1;
            cx.notify();
        }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full();
        if let Some((setup, _)) = &self.setup {
            return root.child(setup.clone());
        }
        let (key, view): (&'static str, AnyView) = match (&self.settings, &self.main) {
            (Some(settings), _) => ("settings", settings.clone().into()),
            (None, Some(main)) => ("main", main.clone().into()),
            (None, None) => return root,
        };
        root.child(div().size_full().child(view).with_animation(
            (key, self.shown),
            Animation::new(Duration::from_millis(450)).with_easing(ease_out_quint()),
            |view, t| view.opacity(t),
        ))
    }
}
