pub(crate) trait Choice: Copy + PartialEq + Default + 'static {
    fn all() -> &'static [Self];
    fn slug(self) -> &'static str;
    fn label(self) -> &'static str;

    fn from_slug(slug: &str) -> Self {
        Self::all()
            .iter()
            .copied()
            .find(|choice| choice.slug() == slug)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;
    use std::fmt::Debug;

    use super::*;
    use crate::settings::{font::FontFamily, theme::Theme};

    fn assert_slugs_round_trip<T: Choice + Debug>() {
        for &choice in T::all() {
            assert_eq!(T::from_slug(choice.slug()), choice);
        }

        assert_eq!(T::from_slug("no-such-slug"), T::default());

        let slugs: HashSet<&str> = T::all().iter().map(|choice| choice.slug()).collect();
        assert_eq!(
            slugs.len(),
            T::all().len(),
            "the picker marks the selected option by slug, so a shared slug would tick the wrong row",
        );
    }

    #[test]
    fn every_choice_survives_a_slug_round_trip_and_the_slugs_are_distinct() {
        assert_slugs_round_trip::<Theme>();
        assert_slugs_round_trip::<FontFamily>();
    }
}
