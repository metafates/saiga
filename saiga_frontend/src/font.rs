#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Font {
    pub family: Family,
    pub weight: Weight,
    pub stretch: Stretch,
    pub style: Style,
}

impl Font {
    /// A non-monospaced sans-serif font with normal [`Weight`].
    pub const DEFAULT: Font = Font {
        family: Family::SansSerif,
        weight: Weight::Normal,
        stretch: Stretch::Normal,
        style: Style::Normal,
    };

    /// A monospaced font with normal [`Weight`].
    pub const MONOSPACE: Font = Font {
        family: Family::Monospace,
        ..Self::DEFAULT
    };

    /// Creates a non-monospaced [`Font`] with the given [`Family::Name`] and
    /// normal [`Weight`].
    pub const fn with_name(name: &'static str) -> Self {
        Font {
            family: Family::Name(name),
            ..Self::DEFAULT
        }
    }

    pub fn attributes(&self) -> glyphon::Attrs {
        glyphon::Attrs::new()
            .family(self.family.into())
            .weight(self.weight.into())
            .stretch(self.stretch.into())
            .style(self.style.into())
    }
}

/// A font family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// The name of a font family of choice.
    Name(&'static str),

    /// Serif fonts represent the formal text style for a script.
    Serif,

    /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low
    /// contrast and have stroke endings that are plain — without any flaring,
    /// cross stroke, or other ornamentation.
    #[default]
    SansSerif,

    /// Glyphs in cursive fonts generally use a more informal script style, and
    /// the result looks more like handwritten pen or brush writing than printed
    /// letterwork.
    Cursive,

    /// Fantasy fonts are primarily decorative or expressive fonts that contain
    /// decorative or expressive representations of characters.
    Fantasy,

    /// The sole criterion of a monospace font is that all glyphs have the same
    /// fixed width.
    Monospace,
}

impl From<Family> for glyphon::Family<'_> {
    fn from(value: Family) -> Self {
        match value {
            Family::Name(name) => glyphon::Family::Name(name),
            Family::Serif => glyphon::Family::Serif,
            Family::SansSerif => glyphon::Family::SansSerif,
            Family::Cursive => glyphon::Family::Cursive,
            Family::Fantasy => glyphon::Family::Fantasy,
            Family::Monospace => glyphon::Family::Monospace,
        }
    }
}

/// The weight of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl From<Weight> for glyphon::Weight {
    fn from(value: Weight) -> Self {
        match value {
            Weight::Thin => glyphon::Weight::THIN,
            Weight::ExtraLight => glyphon::Weight::EXTRA_LIGHT,
            Weight::Light => glyphon::Weight::LIGHT,
            Weight::Normal => glyphon::Weight::NORMAL,
            Weight::Medium => glyphon::Weight::MEDIUM,
            Weight::Semibold => glyphon::Weight::SEMIBOLD,
            Weight::Bold => glyphon::Weight::BOLD,
            Weight::ExtraBold => glyphon::Weight::EXTRA_BOLD,
            Weight::Black => glyphon::Weight::BLACK,
        }
    }
}

/// The width of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Stretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl From<Stretch> for glyphon::Stretch {
    fn from(value: Stretch) -> Self {
        match value {
            Stretch::UltraCondensed => glyphon::Stretch::UltraCondensed,
            Stretch::ExtraCondensed => glyphon::Stretch::ExtraCondensed,
            Stretch::Condensed => glyphon::Stretch::Condensed,
            Stretch::SemiCondensed => glyphon::Stretch::SemiCondensed,
            Stretch::Normal => glyphon::Stretch::Normal,
            Stretch::SemiExpanded => glyphon::Stretch::SemiExpanded,
            Stretch::Expanded => glyphon::Stretch::Expanded,
            Stretch::ExtraExpanded => glyphon::Stretch::ExtraExpanded,
            Stretch::UltraExpanded => glyphon::Stretch::UltraExpanded,
        }
    }
}

/// The style of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Style {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl From<Style> for glyphon::Style {
    fn from(value: Style) -> Self {
        match value {
            Style::Normal => glyphon::Style::Normal,
            Style::Italic => glyphon::Style::Italic,
            Style::Oblique => glyphon::Style::Oblique,
        }
    }
}
