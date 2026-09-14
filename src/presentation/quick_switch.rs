//! Portable quick-switch panel geometry using already compiled styles and text.
use crate::api::overlay::{OverlayLabel, OverlayText};
use crate::api::style::{QuickSwitchPosition, QuickSwitchStyles};
use crate::api::{OverlayScene, Point, Rect};

/// Proportional caption advance estimate, cached when the panel opens.
pub(crate) fn caption_width(text: &str) -> f64 {
    text.chars()
        .map(|ch| match ch {
            'i' | 'l' | 'I' => 0.25,
            'r' | 't' | 'f' | ' ' => 0.35,
            'm' | 'w' => 0.8,
            'M' | 'W' => 0.9,
            '_' => 0.5,
            ch if ch.is_ascii_lowercase() => 0.56,
            ch if ch.is_ascii() => 0.6,
            _ => 1.0,
        })
        .sum::<f64>()
        + 0.15
}

/// Font-relative estimates computed once when the candidate list changes.
#[derive(Clone, Copy, Default)]
pub(crate) struct CaptionMetrics {
    max_width: f64,
    mean_width: f64,
}

impl CaptionMetrics {
    pub(crate) fn for_rows(rows: &[OverlayText]) -> Self {
        if rows.is_empty() {
            let width = caption_width("No available modes");
            return Self {
                max_width: width,
                mean_width: width,
            };
        }
        let mut max_width: f64 = 0.0;
        let mut sum = 0.0;
        for row in rows {
            let width = caption_width(row);
            max_width = max_width.max(width);
            sum += width;
        }
        Self {
            max_width,
            mean_width: sum / rows.len() as f64,
        }
    }

    fn gutters(self, font_size: f64, padding: f64) -> (f64, f64) {
        let gutter = padding + font_size * 0.35;
        // Left-aligned short rows place the average row midpoint left of the
        // longest row's midpoint. Correct a quarter of that difference, keeping
        // at least three quarters of the right gutter to avoid overcorrection.
        let shift =
            ((self.max_width - self.mean_width).max(0.0) * font_size / 8.0).min(gutter * 0.25);
        (gutter + shift, gutter - shift)
    }
}

pub(crate) struct Panel<'a> {
    pub rows: &'a [OverlayText],
    pub text_metrics: CaptionMetrics,
    pub styles: &'a QuickSwitchStyles,
    pub position: QuickSwitchPosition,
    pub screen: Rect,
    pub scale: f64,
    pub window: Option<Rect>,
    pub cursor: Point,
}
impl Panel<'_> {
    pub(crate) fn append_to(&self, scene: &mut OverlayScene) {
        let bounds = self.screen;
        let style = &self.styles.panel;
        // AppKit uses logical points; Retina scale only affects rasterization.
        // Only Windows needs DPI-scaled geometry and compact-label compensation.
        let scale = super::label_scale(self.scale);
        // Use the same compact-label estimate as fit_panel_label so horizontal
        // keycap padding survives layout instead of being clipped by its cell.
        let key_width =
            (self.styles.key.font_size * 0.75 + self.styles.key.padding_x * 2.0) * scale;
        let gap = style.font_size * 0.3 * scale;
        let (left_padding, right_padding) =
            self.text_metrics.gutters(style.font_size, style.padding_x);
        let (left_padding, right_padding) = (left_padding * scale, right_padding * scale);
        let row_height = style.font_size * 1.8 * scale;
        let width = ((self.text_metrics.max_width * style.font_size * scale)
            + left_padding
            + right_padding
            + key_width
            + gap)
            .min(bounds.width);
        let height = (self.rows.len().max(1) as f64 * row_height + style.padding_y * scale * 2.)
            .min(bounds.height);
        let center = match self.position {
            QuickSwitchPosition::Screen => bounds.center(),
            QuickSwitchPosition::Window => self.window.unwrap_or(bounds).center(),
            QuickSwitchPosition::Mouse => Point::new(
                self.cursor.x + width / 2. + 12.,
                scene
                    .indicator
                    .as_ref()
                    .map_or(self.cursor.y + 28., |label| {
                        label.position.y + label.style.font_size * 1.5 + 8.
                    })
                    + height / 2.,
            ),
        };
        let rect = Rect::new(
            (center.x - width / 2.)
                .clamp(bounds.left(), (bounds.right() - width).max(bounds.left())),
            (center.y - height / 2.)
                .clamp(bounds.top(), (bounds.bottom() - height).max(bounds.top())),
            width,
            height,
        );
        let mut label = OverlayLabel::new("", rect, style.clone());
        label.z_index = 10000;
        label.fit_to_text = false;
        scene.labels.push(label);
        let x = rect.x + left_padding;
        let caption_x = x + key_width + gap;
        const KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
        for (index, (text, key)) in self.rows.iter().zip(KEYS).enumerate() {
            let y = rect.y + style.padding_y * scale + index as f64 * row_height;
            let mut badge = OverlayLabel::new(
                key,
                Rect::new(x, y + (row_height - key_width) / 2., key_width, key_width),
                self.styles.key.clone(),
            );
            badge.z_index = 10001;
            badge.fit_to_text = false;
            fit_panel_label(&mut badge, scale);
            scene.labels.push(badge);
            let mut caption = OverlayLabel::new(
                text.clone(),
                Rect::new(
                    caption_x,
                    y,
                    (rect.right() - right_padding - caption_x).max(0.),
                    row_height,
                ),
                self.styles.caption.clone(),
            );
            caption.z_index = 10002;
            caption.fit_to_text = false;
            fit_panel_label(&mut caption, scale);
            scene.labels.push(caption);
        }
        if self.rows.is_empty() {
            let mut empty = OverlayLabel::new(
                "No available modes",
                Rect::new(
                    x,
                    rect.y + style.padding_y * scale,
                    (rect.width - style.padding_x * scale * 2.).max(0.),
                    row_height,
                ),
                self.styles.caption.clone(),
            );
            empty.z_index = 10002;
            empty.fit_to_text = false;
            fit_panel_label(&mut empty, scale);
            scene.labels.push(empty);
        }
        // Preserve any other screen content already included by the active mode.
        scene.clip = scene.clip.map(|clip| clip.union(&bounds));
    }
}

// Match the native compact-label scaling used by the key-help panel.
fn fit_panel_label(label: &mut OverlayLabel, scale: f64) {
    let cell = label.rect;
    let style = &label.style;
    let width = (style.font_size * 0.75 * label.text.chars().count().max(1) as f64
        + style.padding_x * 2.0)
        .min(cell.width / scale);
    let height = style.font_size * 1.4 + style.padding_y * 2.0;
    let left = if style.text_alignment == crate::api::overlay::TextAlignment::Left {
        cell.x
    } else {
        cell.center().x - width * scale / 2.0
    };
    label.rect = Rect::new(
        left.round() + ((width * scale).round() - width) / 2.0,
        cell.center().y - height / 2.0,
        width,
        height,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proportional_mode_names_do_not_reserve_monospace_columns() {
        assert!(caption_width("window_restore") < 14.0 * 0.65);
        assert!(caption_width("iiii") < caption_width("wwww"));
    }
    #[test]
    fn optical_gutters_follow_row_lengths_without_squeezing_longest_caption() {
        for names in [
            vec!["window_restore"],
            vec!["window", "window"],
            vec!["grid", "ui_hint", "window_restore"],
            vec!["i", "i", "i", "i", "a_very_long_custom_mode_name"],
            vec![],
        ] {
            let rows: Vec<OverlayText> = names.into_iter().map(Into::into).collect();
            let metrics = CaptionMetrics::for_rows(&rows);
            let (left, right) = metrics.gutters(28.0, 10.0);
            // Rebalance existing whitespace without enlarging the panel or
            // reducing the longest caption's allocated width.
            assert!((left + right - 39.6).abs() < 0.01);
            assert!(right >= 14.85 - 0.01);
            let original_bias = (metrics.max_width - metrics.mean_width) * 28.0 / 2.0;
            let corrected_bias = original_bias - (left - right) / 2.0;
            if original_bias > 0.01 {
                assert!(corrected_bias >= 0.0 && corrected_bias < original_bias);
                assert!(left > right);
            } else {
                assert_eq!(left, right);
            }
        }
    }
    #[test]
    fn retina_panel_uses_platform_coordinates_and_balanced_gutters() {
        let rows: Vec<OverlayText> = [
            "grid",
            "window",
            "ui_hint",
            "recursive_grid",
            "window_quick",
            "window_editor",
            "window_restore",
            "window_tab",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let styles = QuickSwitchStyles::new(crate::api::overlay::LabelStyle {
            font_size: 28.0,
            padding_x: 10.0,
            padding_y: 6.0,
            ..Default::default()
        });
        let mut scene = OverlayScene::default();
        Panel {
            rows: &rows,
            text_metrics: CaptionMetrics::for_rows(&rows),
            styles: &styles,
            position: QuickSwitchPosition::Screen,
            screen: Rect::new(0.0, 0.0, 1440.0, 900.0),
            scale: 2.0,
            window: None,
            cursor: Point::default(),
        }
        .append_to(&mut scene);
        // Retina backing pixels must not double the 415.2-point AppKit panel.
        let expected_scale = if cfg!(target_os = "windows") {
            2.0
        } else {
            1.0
        };
        let panel = scene.labels[0].rect;
        assert!((panel.height - (8.0 * 28.0 * 1.8 + 12.0) * expected_scale).abs() < 0.01);
        let rendered = |label: &OverlayLabel| {
            if cfg!(target_os = "windows") {
                crate::api::overlay::scaled_label_geometry(
                    &label.text,
                    label.rect,
                    &label.style,
                    expected_scale,
                )
                .0
            } else {
                label.rect
            }
        };
        let key = rendered(&scene.labels[1]);
        let longest = rendered(&scene.labels[14]);
        assert!(key.left() - panel.left() > panel.right() - longest.right());
        assert!(panel.right() - longest.right() >= 9.0 * expected_scale);
        let caption_left = rendered(&scene.labels[2]).left();
        for row in scene.labels[1..].chunks_exact(2) {
            let caption = rendered(&row[1]);
            assert!((caption.left() - caption_left).abs() < 1.0);
            assert!(caption.bottom() <= panel.bottom());
        }
    }
    #[test]
    fn dpi_scaled_rows_keep_the_same_font_and_do_not_overlap() {
        let rows = vec![
            "normal".into(),
            "recursive_grid".into(),
            "window_restore".into(),
        ];
        let styles = QuickSwitchStyles::new(crate::api::overlay::LabelStyle {
            font_size: 28.0,
            padding_x: 6.0,
            ..Default::default()
        });
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let mut scene = OverlayScene::default();
            Panel {
                rows: &rows,
                text_metrics: CaptionMetrics::for_rows(&rows),
                styles: &styles,
                position: QuickSwitchPosition::Screen,
                screen: Rect::new(0.0, 0.0, 1920.0, 1080.0),
                scale,
                window: None,
                cursor: Point::new(0.0, 0.0),
            }
            .append_to(&mut scene);
            let scale = super::super::label_scale(scale);
            let mut bottom = 0.0;
            let mut left = None;
            for row in scene.labels[1..].chunks_exact(2) {
                let (key, key_scale) = crate::api::overlay::scaled_label_geometry(
                    &row[0].text,
                    row[0].rect,
                    &row[0].style,
                    scale,
                );
                let (caption, caption_scale) = crate::api::overlay::scaled_label_geometry(
                    &row[1].text,
                    row[1].rect,
                    &row[1].style,
                    scale,
                );
                assert_eq!(key_scale, scale);
                assert_eq!(caption_scale, scale);
                assert!(key.right() <= caption.left());
                assert!(key.width >= (row[0].style.font_size * 0.99 * scale).floor());
                assert!(key.top().min(caption.top()) >= bottom);
                assert_eq!(*left.get_or_insert(caption.left()), caption.left());
                assert!(caption.right() <= scene.labels[0].rect.right());
                bottom = key.bottom().max(caption.bottom());
                assert_eq!(row[1].style.border_width, 0.0);
            }
        }
    }
    #[test]
    fn positions_use_current_screen_and_clamp_mouse_at_edges() {
        let rows = vec!["normal".into(), "window".into()];
        let styles = QuickSwitchStyles::new(crate::api::overlay::LabelStyle::default());
        let mut panel = Panel {
            rows: &rows,
            text_metrics: CaptionMetrics::for_rows(&rows),
            styles: &styles,
            position: QuickSwitchPosition::Screen,
            scale: 1.0,
            screen: Rect::new(-1920., 0., 1920., 1080.),
            window: Some(Rect::new(-1000., 100., 500., 500.)),
            cursor: Point::new(-1., 1079.),
        };
        let mut scene = OverlayScene::default();
        panel.append_to(&mut scene);
        assert_eq!(scene.labels[0].rect.center(), panel.screen.center());
        panel.position = QuickSwitchPosition::Window;
        panel.append_to(&mut scene);
        assert_eq!(
            scene.labels[5].rect.center(),
            panel.window.unwrap().center()
        );
        panel.position = QuickSwitchPosition::Mouse;
        panel.append_to(&mut scene);
        let rect = scene.labels[10].rect;
        assert!(rect.right() <= panel.screen.right() && rect.bottom() <= panel.screen.bottom());
        assert_eq!(scene.labels[1].text.as_str(), "1");
        assert_eq!(scene.labels[3].text.as_str(), "2");
        assert_eq!(scene.labels[2].rect.x, scene.labels[4].rect.x);
        assert_eq!(
            scene.labels[2].style.text_alignment,
            crate::api::overlay::TextAlignment::Left
        );
        assert_eq!(
            scene.labels[1].rect.center().y,
            scene.labels[2].rect.center().y
        );
    }
}
