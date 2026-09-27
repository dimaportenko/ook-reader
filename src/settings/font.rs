use crate::settings::choice::Choice;

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
}

impl Choice for FontFamily {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn slug(self) -> &'static str {
        match self {
            FontFamily::Publisher => "publisher",
            FontFamily::OldStyle => "old-style",
            FontFamily::Modern => "modern",
            FontFamily::Sans => "sans",
            FontFamily::Humanist => "humanist",
        }
    }

    fn label(self) -> &'static str {
        match self {
            FontFamily::Publisher => "Publisher",
            FontFamily::OldStyle => "Old style",
            FontFamily::Modern => "Modern",
            FontFamily::Sans => "Sans",
            FontFamily::Humanist => "Humanist",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn each_font_family_has_a_reader_facing_label() {
        assert_eq!(
            FontFamily::ALL.map(FontFamily::label),
            ["Publisher", "Old style", "Modern", "Sans", "Humanist"]
        );
    }
}
