//! Management of the Stylo [`Device`]: constructing it from shell state and
//! coalescing changes to it (viewport resizes, zoom, color-scheme and media
//! type changes) so that the stylist device is rebuilt at most once per
//! [`resolve`](crate::BaseDocument::resolve), no matter how many changes
//! occurred since the last resolve.

use std::sync::{Arc, Mutex};

use bitflags::bitflags;
use blitz_traits::shell::{ColorScheme, Viewport};
use parley::FontContext;
use selectors::matching::QuirksMode;
use style::device::Device;
use style::media_queries::MediaType;
use style::properties::ComputedValues;
use style::properties::style_structs::Font;
use style::queries::values::{PrefersColorScheme, PrefersContrast, PrefersReducedMotion};
use style::servo::media_features::PointerCapabilities;

use crate::font_metrics::BlitzFontMetricsProvider;

bitflags! {
    /// The set of changes to the [`Device`] that have accumulated since the
    /// stylist device was last rebuilt. Only the *latest* values matter (they
    /// live on [`BaseDocument`](crate::BaseDocument)); these flags record
    /// which kinds of change occurred so that the flush can do the right
    /// amount of invalidation work.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub(crate) struct DeviceChanges: u8 {
        /// The CSS pixel viewport size changed (window resize, zoom, or hidpi
        /// scale change).
        const VIEWPORT_SIZE = 0b0000_0001;
        /// The total scale factor (hidpi scale * zoom) changed. Requires
        /// invalidating cached inline layouts (text is shaped at a specific
        /// scale) and a redraw.
        const SCALE = 0b0000_0010;
        /// The preferred color scheme changed.
        const COLOR_SCHEME = 0b0000_0100;
        /// The media type (e.g. screen/print) changed.
        const MEDIA_TYPE = 0b0000_1000;
    }
}

impl DeviceChanges {
    /// Compute the [`DeviceChanges`] implied by moving from viewport `old` to
    /// viewport `new`.
    pub(crate) fn from_viewports(old: &Viewport, new: &Viewport) -> Self {
        let mut changes = Self::empty();
        if old.logical_size() != new.logical_size() {
            changes |= Self::VIEWPORT_SIZE;
        }
        if old.scale_f64() != new.scale_f64() {
            changes |= Self::SCALE;
        }
        if old.color_scheme != new.color_scheme {
            changes |= Self::COLOR_SCHEME;
        }
        changes
    }
}

/// What the user prefers, as `prefers-reduced-motion` and
/// `prefers-contrast` media queries see it (Media Queries 5 §11.3, §11.4).
/// The embedder knows these from its own settings or the system; the
/// default is "no preference" for both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MediaPreferences {
    pub reduced_motion: bool,
    pub contrast: Contrast,
}

/// The `prefers-contrast` values (Media Queries 5 §11.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Contrast {
    #[default]
    NoPreference,
    More,
    Less,
    Custom,
}

pub(crate) fn make_device(
    viewport: &Viewport,
    media_type: MediaType,
    font_ctx: Arc<Mutex<FontContext>>,
    media_device_pixel_ratio: Option<f32>,
    preferences: MediaPreferences,
) -> Device {
    let (width, height) = viewport.logical_size();
    let viewport_size = euclid::Size2D::new(width, height);
    let scale = media_device_pixel_ratio.unwrap_or(viewport.scale());
    let device_size = euclid::Size2D::new(width, height) * scale;
    let device_pixel_ratio = euclid::Scale::new(scale);

    let mut device = Device::new(
        media_type,
        QuirksMode::NoQuirks,
        viewport_size,
        device_size,
        device_pixel_ratio,
        Box::new(BlitzFontMetricsProvider { font_ctx }),
        ComputedValues::initial_values_with_font_override(Font::initial_values()),
        match viewport.color_scheme {
            ColorScheme::Light => PrefersColorScheme::Light,
            ColorScheme::Dark => PrefersColorScheme::Dark,
        },
        PointerCapabilities::default(),
        PointerCapabilities::default(),
    );
    device.set_prefers_reduced_motion(if preferences.reduced_motion {
        PrefersReducedMotion::Reduce
    } else {
        PrefersReducedMotion::NoPreference
    });
    device.set_prefers_contrast(match preferences.contrast {
        Contrast::NoPreference => PrefersContrast::NoPreference,
        Contrast::More => PrefersContrast::More,
        Contrast::Less => PrefersContrast::Less,
        Contrast::Custom => PrefersContrast::Custom,
    });
    device
}
