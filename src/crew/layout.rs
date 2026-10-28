//! Shared exclusion area for third-person combat overlays and the floating crew portrait.
use super::*;
pub(crate) const COMBAT_WIDTH: f32 = 284.;
pub(crate) const COMBAT_TOP: f32 = 165.;
pub(crate) fn portrait_bounds(
    crew: Option<&Crew>,
    flight: &Flight,
    view: &ViewState,
    combat: &Combat,
    size: Vec2,
) -> Option<Rect> {
    let crew = crew?;
    if crew.editor
        || view.missile_follow.is_some()
        || !flight_for(owner(view, crew), flight, combat).is_some_and(supported)
    {
        return None;
    }
    let at = if crew.panel.x < 0. {
        size - crew.size - Vec2::splat(18.)
    } else {
        crew.panel
    };
    Some(Rect::from_corners(at, at + crew.size))
}
/// Keep the entire combat column horizontally outside the portrait, including
/// when the portrait is resized or dragged into the column's usual position.
pub(crate) fn combat_right(size: Vec2, portrait: Rect) -> f32 {
    let preferred = 340_f32.min((size.x - COMBAT_WIDTH - 28.).max(28.));
    let left = size.x - preferred - COMBAT_WIDTH;
    let right = size.x - preferred;
    if right <= portrait.min.x - 12. || left >= portrait.max.x + 12. {
        return preferred;
    }
    [
        size.x - portrait.min.x + 12.,
        size.x - portrait.max.x - 12. - COMBAT_WIDTH,
    ]
    .into_iter()
    .filter(|right| *right >= 28. && size.x - right - COMBAT_WIDTH >= 28.)
    .min_by(|a, b| (a - preferred).abs().total_cmp(&(b - preferred).abs()))
    .unwrap_or(preferred)
}
pub(crate) fn weather_right(size: Vec2, column: Option<f32>) -> f32 {
    column
        .filter(|right| {
            size.x - 28. - 292. < size.x - right && size.x - right - COMBAT_WIDTH < size.x - 28.
        })
        .map_or(28., |right| right + COMBAT_WIDTH + 12.)
}
pub(crate) fn threat_positions(
    size: Vec2,
    simple: bool,
    portrait: Option<Rect>,
    panels_bottom: f32,
) -> (Vec2, Vec2, bool) {
    let old = if simple {
        Vec2::new(size.x - 110., 150.)
    } else {
        Vec2::new(size.x - 245., size.y - 425.)
    };
    if let Some(portrait) = portrait {
        let old_bounds = Rect::from_corners(old - Vec2::new(85., 42.), old + Vec2::new(100., 90.));
        if !simple || !old_bounds.intersect(portrait).is_empty() {
            let edge = size.x - combat_right(size, portrait);
            let centre = Vec2::new(edge - 220., (size.y - 300.).max(panels_bottom + 54.));
            return (centre, centre + Vec2::new(50., -22.), true);
        }
    }
    (old, old + Vec2::new(-85., 46.), false)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combat_panels_and_missile_warning_clear_default_resized_and_moved_portraits() {
        let size = Vec2::new(1440., 900.);
        for (at, extent) in [
            (Vec2::new(1178., 598.), Vec2::new(244., 284.)),
            (Vec2::new(942., 362.), Vec2::new(480., 520.)),
            (Vec2::new(800., 300.), Vec2::new(300., 350.)),
            (Vec2::new(50., 500.), Vec2::new(244., 284.)),
        ] {
            let portrait = Rect::from_corners(at, at + extent);
            let right = combat_right(size, portrait);
            let weather = Vec2::new(size.x - weather_right(size, Some(right)) - 292., 140.);
            let weapon = Vec2::new(size.x - right - COMBAT_WIDTH, COMBAT_TOP);
            assert!(
                Rect::from_corners(weather, weather + Vec2::new(292., 112.))
                    .intersect(Rect::from_corners(
                        weapon,
                        weapon + Vec2::new(COMBAT_WIDTH, 132.)
                    ))
                    .is_empty()
            );
            for (top, height) in [(COMBAT_TOP, 132.), (COMBAT_TOP + 144., 164.)] {
                let p = Vec2::new(size.x - right - COMBAT_WIDTH, top);
                assert!(
                    Rect::from_corners(p, p + Vec2::new(COMBAT_WIDTH, height))
                        .intersect(portrait)
                        .is_empty()
                );
            }
            let (centre, label, docked) =
                threat_positions(size, false, Some(portrait), COMBAT_TOP + 308.);
            assert!(docked);
            assert!(
                Rect::from_corners(centre - Vec2::splat(42.), centre + Vec2::splat(42.))
                    .intersect(portrait)
                    .is_empty()
            );
            assert!(
                Rect::from_corners(label, label + Vec2::new(170., 52.))
                    .intersect(portrait)
                    .is_empty()
            );
        }
    }
    #[test]
    fn hidden_portrait_and_clear_simplified_hud_keep_original_warning_positions() {
        let size = Vec2::new(1440., 900.);
        let (p, _, docked) = threat_positions(size, false, None, 0.);
        assert_eq!(p, Vec2::new(1195., 475.));
        assert!(!docked);
        let portrait = Rect::from_corners(Vec2::new(1178., 598.), Vec2::new(1422., 882.));
        let (p, _, docked) = threat_positions(size, true, Some(portrait), 0.);
        assert_eq!(p, Vec2::new(1330., 150.));
        assert!(!docked);
    }
}
