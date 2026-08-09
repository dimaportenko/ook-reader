#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FontFamily {
    #[default]
    Publisher,
    OldStyle,
    Modern,
    Sans,
    Humanist,
}

impl FontFamily {
    pub(crate) const ALL: [FontFamily; 5] = [
        FontFamily::Publisher,
        FontFamily::OldStyle,
        FontFamily::Modern,
        FontFamily::Sans,
        FontFamily::Humanist,
    ];

    pub(crate) fn stack(self) -> &'static str {
        match self {
            FontFamily::Publisher => "",
            FontFamily::OldStyle => "'Iowan Old Style', 'Sitka Text', Palatino, Georgia, serif",
            FontFamily::Modern => "Athelas, Charter, 'Bitstream Charter', Cambria, serif",
            FontFamily::Sans => "Seravek, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif",
            FontFamily::Humanist => "Frutiger, Calibri, 'Gill Sans', 'Lucida Grande', sans-serif",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            FontFamily::Publisher => "publisher",
            FontFamily::OldStyle => "old-style",
            FontFamily::Modern => "modern",
            FontFamily::Sans => "sans",
            FontFamily::Humanist => "humanist",
        }
    }

    pub(crate) fn from_slug(slug: &str) -> FontFamily {
        match slug {
            "publisher" => FontFamily::Publisher,
            "old-style" => FontFamily::OldStyle,
            "modern" => FontFamily::Modern,
            "sans" => FontFamily::Sans,
            "humanist" => FontFamily::Humanist,
            _ => FontFamily::default(),
        }
    }
}
