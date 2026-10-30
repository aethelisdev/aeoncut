use eframe::egui::{self, Color32, Rect, Stroke};

pub fn draw_pin_icon(painter: &egui::Painter, rect: Rect, color: Color32, pinned: bool) {
    let center = rect.center();
    if pinned {
        // Pinned: Straight vertical pin inserted down
        // 1. Top cap bar
        painter.line_segment(
            [
                center + egui::vec2(-4.0, -5.5),
                center + egui::vec2(4.0, -5.5),
            ],
            Stroke::new(1.8, color),
        );
        // 2. Solid body / barrel
        painter.rect_filled(
            Rect::from_min_max(
                center + egui::vec2(-2.5, -5.5),
                center + egui::vec2(2.5, -0.5),
            ),
            1.0,
            color,
        );
        // 3. Wide brim / collar
        painter.line_segment(
            [
                center + egui::vec2(-5.0, -0.5),
                center + egui::vec2(5.0, -0.5),
            ],
            Stroke::new(2.0, color),
        );
        // 4. Needle tip
        painter.line_segment(
            [center + egui::vec2(0.0, 0.5), center + egui::vec2(0.0, 6.5)],
            Stroke::new(1.6, color),
        );
    } else {
        // Unpinned: 35-degree tilted pin
        let angle = 35.0_f32.to_radians();
        let cos = angle.cos();
        let sin = angle.sin();
        let rot = |dx: f32, dy: f32| egui::vec2(dx * cos - dy * sin, dx * sin + dy * cos);

        // 1. Top cap bar
        painter.line_segment(
            [center + rot(-4.0, -5.5), center + rot(4.0, -5.5)],
            Stroke::new(1.8, color),
        );
        // 2. Angled barrel (quad)
        let p1 = center + rot(-2.5, -5.5);
        let p2 = center + rot(2.5, -5.5);
        let p3 = center + rot(2.5, -0.5);
        let p4 = center + rot(-2.5, -0.5);
        painter.add(egui::Shape::convex_polygon(
            vec![p1, p2, p3, p4],
            color,
            Stroke::NONE,
        ));
        // 3. Wide brim / collar
        painter.line_segment(
            [center + rot(-5.0, -0.5), center + rot(5.0, -0.5)],
            Stroke::new(2.0, color),
        );
        // 4. Needle tip
        painter.line_segment(
            [center + rot(0.0, 0.5), center + rot(0.0, 6.5)],
            Stroke::new(1.6, color),
        );
    }
}
