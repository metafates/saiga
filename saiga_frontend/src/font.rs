#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: Family,
    pub weight: Weight,
    pub stretch: Stretch,
    pub style: Style,
}

impl Default for Font {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Font {
    /// A monospaced font with normal [`Weight`].
    pub const DEFAULT: Font = Font {
        family: Family::Monospace,
        weight: Weight::Normal,
        stretch: Stretch::Normal,
        style: Style::Normal,
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

    /// The sole criterion of a monospace font is that all glyphs have the same
    /// fixed width.
    #[default]
    Monospace,
}

impl From<Family> for glyphon::Family<'_> {
    fn from(value: Family) -> Self {
        match value {
            Family::Name(name) => glyphon::Family::Name(name),
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
