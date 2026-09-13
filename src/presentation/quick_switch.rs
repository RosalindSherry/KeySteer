//! Portable quick-switch panel geometry using already compiled styles and text.
use crate::api::overlay::{OverlayLabel, OverlayText};
use crate::api::style::{QuickSwitchPosition, QuickSwitchStyles};
use crate::api::{OverlayScene, Point, Rect};

/// Proportional caption advance estimate, cached when the panel opens.
pub(crate) fn caption_width(text: &str) -> f64 {
    text.chars()
        .map(|ch| match ch {
            'i' | 'l' | 'I' => 0.28,
            'r' | 't' | 'f' | ' ' => 0.38,
            'm' | 'w' | 'M' | 'W' => 0.9,
            '_' => 0.5,
            ch if ch.is_ascii() => 0.6,
            _ => 1.0,
        })
        .sum::<f64>()
        + 0.15
}

pub(crate) struct Panel<'a> {
    pub rows: &'a [OverlayText],
    pub text_width: f64,
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
        let scale = self.scale.max(1.0);
        let key_width = style.font_size * 0.85 * scale;
        let gap = style.font_size * 0.3 * scale;
        let left_padding = (style.padding_x + style.font_size * 0.55) * scale;
        let right_padding = style.padding_x * 0.5 * scale;
        let row_height = style.font_size * 1.8 * scale;
        let width = ((self.text_width * style.font_size * scale)
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
                text_width: caption_width("window_restore"),
                styles: &styles,
                position: QuickSwitchPosition::Screen,
                screen: Rect::new(0.0, 0.0, 1920.0, 1080.0),
                scale,
                window: None,
                cursor: Point::new(0.0, 0.0),
            }
            .append_to(&mut scene);
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
            text_width: caption_width("window"),
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
