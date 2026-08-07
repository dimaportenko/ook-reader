#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Theme {
    #[default]
    Day,
    Sepia,
    #[allow(dead_code)]
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

    fn declarations(self) -> String {
        let (background, text) = self.colors();

        format!("--USER__backgroundColor: {background}; --USER__textColor: {text};")
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
            "{} background: var(--USER__backgroundColor); \
                color: var(--USER__textColor)",
            self.declarations()
        )
    }
}
