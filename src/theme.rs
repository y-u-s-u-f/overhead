use gpui::{Hsla, WindowAppearance, hsla};

pub struct Theme {
    pub background: Hsla,
    pub card: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub accent: Hsla,
    pub accent_soft: Hsla,
    pub on_accent: Hsla,
    pub live: Hsla,
    pub error: Hsla,
}

impl Theme {
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }

    fn dark() -> Self {
        let white = |a| hsla(0., 0., 1., a);
        Self {
            background: hsla(220. / 360., 0.12, 0.09, 0.62),
            card: white(0.06),
            border: white(0.08),
            text: white(0.94),
            muted: white(0.60),
            faint: white(0.38),
            accent: hsla(36. / 360., 0.96, 0.60, 1.),
            accent_soft: hsla(36. / 360., 0.96, 0.60, 0.14),
            on_accent: hsla(30. / 360., 0.40, 0.08, 1.),
            live: hsla(145. / 360., 0.60, 0.52, 1.),
            error: hsla(4. / 360., 0.85, 0.64, 1.),
        }
    }

    fn light() -> Self {
        let black = |a| hsla(0., 0., 0., a);
        Self {
            background: hsla(220. / 360., 0.20, 0.97, 0.74),
            card: hsla(0., 0., 1., 0.62),
            border: black(0.07),
            text: black(0.88),
            muted: black(0.55),
            faint: black(0.35),
            accent: hsla(30. / 360., 0.92, 0.44, 1.),
            accent_soft: hsla(30. / 360., 0.92, 0.50, 0.13),
            on_accent: hsla(0., 0., 1., 1.),
            live: hsla(145. / 360., 0.62, 0.36, 1.),
            error: hsla(4. / 360., 0.72, 0.50, 1.),
        }
    }
}
