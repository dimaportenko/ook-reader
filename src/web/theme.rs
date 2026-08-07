#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Theme {
    #[default]
    Day,
    Sepia,
    Night,
}

impl Theme {
    pub(crate) fn colors(self) -> (&'static str, &'static str) {
        match self {
            Theme::Day => ("#ffffff", "#121212"),
            Theme::Sepia => ("#faf4e8", "#5b4636"),
            Theme::Night => ("#121212", "#cfcfcf"),
        }
    }

    pub(crate) fn css_vars(self) -> [(&'static str, &'static str); 2] {
        let (background, text) = self.colors();

        [
            ("--USER__backgroundColor", background),
            ("--USER__textColor", text),
        ]
    }

    fn declarations(self) -> String {
        self.css_vars()
            .iter()
            .map(|(name, value)| format!("{name}: {value};"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn vars(self) -> String {
        format!(":root {{ {} }}", self.declarations())
    }

    pub(crate) fn user_layer(self) -> String {
        format!(
            "{}\nbody {{ background: var(--USER__backgroundColor) !important; \
                color: var(--USER__textColor) !important; }}",
            self.vars()
        )
    }

    pub(crate) fn inline_styles(self) -> String {
        format!(
            "{} background-color: var(--USER__backgroundColor); color: var(--USER__textColor)",
            self.declarations()
        )
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Theme::Day => "day",
            Theme::Sepia => "sepia",
            Theme::Night => "night",
        }
    }

    pub(crate) fn from_slug(slug: &str) -> Theme {
        match slug {
            "day" => Theme::Day,
            "sepia" => Theme::Sepia,
            "night" => Theme::Night,
            _ => Theme::default(),
        }
    }
}
