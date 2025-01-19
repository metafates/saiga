use std::{collections::HashMap, mem};

use event::{Event, EventListener};
use log::{debug, trace};
use saiga_vte::ansi::handler::{
    Attribute, Charset, CharsetIndex, Color, Handler, LineClearMode, NamedColor, NamedPrivateMode,
    PrivateMode, Rgb,
};
use unicode_width::UnicodeWidthChar;
use crate::grid::{Dimensions, Grid};

pub mod event;
pub mod grid;
pub mod pty;
pub mod index;
pub mod term;
pub mod selection;

