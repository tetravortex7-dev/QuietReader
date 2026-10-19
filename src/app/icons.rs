use eframe::egui::{self, Color32, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2, Widget};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    OpenFile,
    Folder,
    Recent,
    Toc,
    Fullscreen,
    Windowed,
    Settings,
    Close,
    Trash,
    Book,
    Warning,
    Fonts,
    Appearance,
    AutoScroll,
    Keyboard,
    Plus,
    Minus,
    Play,
    Pause,
    Check,
    Search,
    ChevronUp,
    ChevronDown,
    Speech,
    Ruler,
    Globe,
    ArrowLeft,
    ArrowRight,
    Home,
    Refresh,
    Download,
    Cross,
    Copy,
    Key,
    Bookmark,
    Quote,
    Edit,
    Headphones,
    Moon,
    Crop,
}

pub fn paint_icon(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let w = rect.width();
    let h = rect.height();
    let p = |u: f32, v: f32| -> Pos2 { Pos2::new(rect.min.x + u * w, rect.min.y + v * h) };
    let stroke = Stroke::new(1.4, color);
    let bold_stroke = Stroke::new(1.6, color);
    let thin_stroke = Stroke::new(1.2, color);

    match icon {
        Icon::OpenFile => {
            let outline = vec![
                p(0.22, 0.14),
                p(0.56, 0.14),
                p(0.78, 0.36),
                p(0.78, 0.86),
                p(0.22, 0.86),
            ];
            painter.add(egui::Shape::closed_line(outline, stroke));
            let fold = vec![p(0.56, 0.14), p(0.56, 0.36), p(0.78, 0.36)];
            painter.add(egui::Shape::line(fold, thin_stroke));
            painter.line_segment([p(0.34, 0.52), p(0.66, 0.52)], thin_stroke);
            painter.line_segment([p(0.34, 0.68), p(0.54, 0.68)], thin_stroke);
        }
        Icon::Folder => {
            let tab = vec![
                p(0.14, 0.26),
                p(0.40, 0.26),
                p(0.48, 0.36),
                p(0.86, 0.36),
                p(0.86, 0.80),
                p(0.14, 0.80),
            ];
            painter.add(egui::Shape::closed_line(tab, stroke));
            painter.line_segment([p(0.14, 0.48), p(0.86, 0.48)], thin_stroke);
        }
        Icon::Recent => {
            let center = p(0.50, 0.50);
            painter.circle_stroke(center, w * 0.36, stroke);
            let hands = vec![p(0.50, 0.24), center, p(0.72, 0.50)];
            painter.add(egui::Shape::line(hands, bold_stroke));
            painter.circle_filled(center, 1.2, color);
        }
        Icon::Toc => {
            painter.circle_filled(p(0.20, 0.24), 1.4, color);
            painter.line_segment([p(0.32, 0.24), p(0.82, 0.24)], stroke);

            painter.circle_filled(p(0.32, 0.42), 1.2, color);
            painter.line_segment([p(0.44, 0.42), p(0.78, 0.42)], thin_stroke);

            painter.circle_filled(p(0.32, 0.60), 1.2, color);
            painter.line_segment([p(0.44, 0.60), p(0.72, 0.60)], thin_stroke);

            painter.circle_filled(p(0.20, 0.78), 1.4, color);
            painter.line_segment([p(0.32, 0.78), p(0.82, 0.78)], stroke);
        }
        Icon::Book => {
            let left_page = vec![
                p(0.50, 0.30),
                p(0.44, 0.24),
                p(0.14, 0.24),
                p(0.14, 0.76),
                p(0.44, 0.76),
                p(0.50, 0.82),
            ];
            painter.add(egui::Shape::line(left_page, stroke));

            let right_page = vec![
                p(0.50, 0.30),
                p(0.56, 0.24),
                p(0.86, 0.24),
                p(0.86, 0.76),
                p(0.56, 0.76),
                p(0.50, 0.82),
            ];
            painter.add(egui::Shape::line(right_page, stroke));

            painter.line_segment([p(0.50, 0.30), p(0.50, 0.82)], stroke);
            painter.line_segment([p(0.22, 0.42), p(0.42, 0.42)], thin_stroke);
            painter.line_segment([p(0.22, 0.58), p(0.42, 0.58)], thin_stroke);
            painter.line_segment([p(0.58, 0.42), p(0.78, 0.42)], thin_stroke);
            painter.line_segment([p(0.58, 0.58), p(0.78, 0.58)], thin_stroke);
        }
        Icon::Fullscreen => {
            painter.add(egui::Shape::line(
                vec![p(0.16, 0.38), p(0.16, 0.16), p(0.38, 0.16)],
                bold_stroke,
            ));
            painter.add(egui::Shape::line(
                vec![p(0.62, 0.16), p(0.84, 0.16), p(0.84, 0.38)],
                bold_stroke,
            ));
            painter.add(egui::Shape::line(
                vec![p(0.16, 0.62), p(0.16, 0.84), p(0.38, 0.84)],
                bold_stroke,
            ));
            painter.add(egui::Shape::line(
                vec![p(0.62, 0.84), p(0.84, 0.84), p(0.84, 0.62)],
                bold_stroke,
            ));
        }
        Icon::Windowed => {
            let back = vec![
                p(0.36, 0.34),
                p(0.36, 0.16),
                p(0.84, 0.16),
                p(0.84, 0.64),
                p(0.66, 0.64),
            ];
            painter.add(egui::Shape::line(back, stroke));
            painter.line_segment([p(0.36, 0.28), p(0.84, 0.28)], thin_stroke);

            let front = Rect::from_min_max(p(0.16, 0.34), p(0.66, 0.84));
            painter.rect_stroke(front, 1.5, stroke, StrokeKind::Inside);
            painter.line_segment([p(0.16, 0.46), p(0.66, 0.46)], thin_stroke);
            painter.circle_filled(p(0.24, 0.40), 1.0, color);
            painter.circle_filled(p(0.32, 0.40), 1.0, color);
        }
        Icon::Settings => {
            let center = p(0.50, 0.50);
            let r_in = w * 0.28;
            let r_out = w * 0.43;
            let num_teeth = 6;
            let step = (2.0 * PI) / (num_teeth as f32);
            let mut pts = Vec::with_capacity(num_teeth * 4);

            for i in 0..num_teeth {
                let mid_angle = (i as f32) * step - PI * 0.5;
                let a0 = mid_angle - 0.26;
                let a1 = mid_angle - 0.13;
                let a2 = mid_angle + 0.13;
                let a3 = mid_angle + 0.26;

                pts.push(Pos2::new(
                    center.x + a0.cos() * r_in,
                    center.y + a0.sin() * r_in,
                ));
                pts.push(Pos2::new(
                    center.x + a1.cos() * r_out,
                    center.y + a1.sin() * r_out,
                ));
                pts.push(Pos2::new(
                    center.x + a2.cos() * r_out,
                    center.y + a2.sin() * r_out,
                ));
                pts.push(Pos2::new(
                    center.x + a3.cos() * r_in,
                    center.y + a3.sin() * r_in,
                ));
            }

            painter.add(egui::Shape::closed_line(pts, stroke));
            painter.circle_stroke(center, w * 0.13, stroke);
        }
        Icon::Close => {
            painter.line_segment([p(0.74, 0.50), p(0.24, 0.50)], bold_stroke);
            painter.add(egui::Shape::line(
                vec![p(0.48, 0.26), p(0.24, 0.50), p(0.48, 0.74)],
                bold_stroke,
            ));
        }
        Icon::Trash => {
            let handle = vec![p(0.38, 0.26), p(0.38, 0.16), p(0.62, 0.16), p(0.62, 0.26)];
            painter.add(egui::Shape::line(handle, thin_stroke));
            painter.line_segment([p(0.18, 0.26), p(0.82, 0.26)], bold_stroke);

            let body = vec![p(0.24, 0.30), p(0.28, 0.82), p(0.72, 0.82), p(0.76, 0.30)];
            painter.add(egui::Shape::line(body, stroke));

            painter.line_segment([p(0.42, 0.38), p(0.42, 0.72)], thin_stroke);
            painter.line_segment([p(0.58, 0.38), p(0.58, 0.72)], thin_stroke);
        }
        Icon::Warning => {
            let triangle = vec![p(0.50, 0.14), p(0.88, 0.82), p(0.12, 0.82)];
            painter.add(egui::Shape::closed_line(triangle, bold_stroke));
            painter.line_segment([p(0.50, 0.36), p(0.50, 0.58)], bold_stroke);
            painter.circle_filled(p(0.50, 0.70), 1.3, color);
        }
        Icon::Fonts => {
            painter.line_segment([p(0.14, 0.22), p(0.56, 0.22)], bold_stroke);
            painter.line_segment([p(0.35, 0.22), p(0.35, 0.80)], bold_stroke);
            painter.line_segment([p(0.25, 0.80), p(0.45, 0.80)], thin_stroke);

            painter.line_segment([p(0.54, 0.42), p(0.84, 0.42)], stroke);
            let small_t = vec![p(0.68, 0.30), p(0.68, 0.76), p(0.82, 0.78)];
            painter.add(egui::Shape::line(small_t, stroke));
        }
        Icon::Appearance => {
            let palette = vec![
                p(0.50, 0.16),
                p(0.78, 0.24),
                p(0.86, 0.48),
                p(0.78, 0.72),
                p(0.62, 0.68),
                p(0.54, 0.82),
                p(0.30, 0.82),
                p(0.14, 0.58),
                p(0.22, 0.28),
            ];
            painter.add(egui::Shape::closed_line(palette, stroke));
            painter.circle_stroke(p(0.62, 0.56), w * 0.08, thin_stroke);
            painter.circle_filled(p(0.32, 0.38), 1.3, color);
            painter.circle_filled(p(0.50, 0.30), 1.3, color);
            painter.circle_filled(p(0.34, 0.58), 1.3, color);
        }
        Icon::AutoScroll => {
            let chev1 = vec![p(0.22, 0.30), p(0.50, 0.50), p(0.78, 0.30)];
            let chev2 = vec![p(0.22, 0.52), p(0.50, 0.72), p(0.78, 0.52)];
            painter.add(egui::Shape::line(chev1, bold_stroke));
            painter.add(egui::Shape::line(chev2, bold_stroke));
        }
        Icon::Keyboard => {
            let frame = Rect::from_min_max(p(0.12, 0.24), p(0.88, 0.76));
            painter.rect_stroke(frame, 2.5, stroke, StrokeKind::Inside);

            painter.line_segment([p(0.24, 0.40), p(0.34, 0.40)], stroke);
            painter.line_segment([p(0.45, 0.40), p(0.55, 0.40)], stroke);
            painter.line_segment([p(0.66, 0.40), p(0.76, 0.40)], stroke);

            let space = vec![p(0.26, 0.54), p(0.26, 0.60), p(0.74, 0.60), p(0.74, 0.54)];
            painter.add(egui::Shape::line(space, stroke));
        }
        Icon::Plus => {
            painter.line_segment([p(0.20, 0.50), p(0.80, 0.50)], Stroke::new(1.8, color));
            painter.line_segment([p(0.50, 0.20), p(0.50, 0.80)], Stroke::new(1.8, color));
        }
        Icon::Minus => {
            painter.line_segment([p(0.20, 0.50), p(0.80, 0.50)], Stroke::new(1.8, color));
        }
        Icon::Play => {
            let tri = vec![p(0.30, 0.22), p(0.78, 0.50), p(0.30, 0.78)];
            painter.add(egui::Shape::convex_polygon(tri, color, Stroke::NONE));
        }
        Icon::Pause => {
            painter.rect_filled(Rect::from_min_max(p(0.28, 0.22), p(0.42, 0.78)), 1.5, color);
            painter.rect_filled(Rect::from_min_max(p(0.58, 0.22), p(0.72, 0.78)), 1.5, color);
        }
        Icon::Check => {
            let check = vec![p(0.18, 0.52), p(0.38, 0.74), p(0.82, 0.26)];
            painter.add(egui::Shape::line(check, Stroke::new(1.8, color)));
        }
        Icon::Search => {
            let center = p(0.42, 0.42);
            let r = w * 0.24;
            painter.circle_stroke(center, r, stroke);
            let handle = vec![p(0.59, 0.59), p(0.82, 0.82)];
            painter.add(egui::Shape::line(handle, Stroke::new(1.8, color)));
        }
        Icon::ChevronUp => {
            let chev = vec![p(0.24, 0.62), p(0.50, 0.36), p(0.76, 0.62)];
            painter.add(egui::Shape::line(chev, bold_stroke));
        }
        Icon::ChevronDown => {
            let chev = vec![p(0.24, 0.38), p(0.50, 0.64), p(0.76, 0.38)];
            painter.add(egui::Shape::line(chev, bold_stroke));
        }
        Icon::Speech => {
            let cone = vec![
                p(0.14, 0.38),
                p(0.32, 0.38),
                p(0.52, 0.22),
                p(0.52, 0.78),
                p(0.32, 0.62),
                p(0.14, 0.62),
            ];
            painter.add(egui::Shape::convex_polygon(cone, color, Stroke::NONE));
            let wave1 = vec![
                p(0.64, 0.37),
                p(0.70, 0.43),
                p(0.72, 0.50),
                p(0.70, 0.57),
                p(0.64, 0.63),
            ];
            painter.add(egui::Shape::line(wave1, Stroke::new(1.6, color)));
            let wave2 = vec![
                p(0.78, 0.26),
                p(0.86, 0.37),
                p(0.89, 0.50),
                p(0.86, 0.63),
                p(0.78, 0.74),
            ];
            painter.add(egui::Shape::line(wave2, Stroke::new(1.6, color)));
        }
        Icon::Ruler => {
            let outline = vec![p(0.12, 0.32), p(0.88, 0.32), p(0.88, 0.68), p(0.12, 0.68)];
            painter.add(egui::Shape::closed_line(outline, stroke));
            painter.line_segment([p(0.24, 0.32), p(0.24, 0.52)], thin_stroke);
            painter.line_segment([p(0.38, 0.32), p(0.38, 0.46)], thin_stroke);
            painter.line_segment([p(0.52, 0.32), p(0.52, 0.52)], thin_stroke);
            painter.line_segment([p(0.66, 0.32), p(0.66, 0.46)], thin_stroke);
            painter.line_segment([p(0.78, 0.32), p(0.78, 0.52)], thin_stroke);
        }
        Icon::Globe => {
            let center = p(0.50, 0.50);
            painter.circle_stroke(center, w * 0.36, stroke);
            painter.line_segment([p(0.14, 0.50), p(0.86, 0.50)], thin_stroke);
            painter.line_segment([p(0.50, 0.14), p(0.50, 0.86)], thin_stroke);
            let arc_left = vec![
                p(0.50, 0.14),
                p(0.32, 0.30),
                p(0.28, 0.50),
                p(0.32, 0.70),
                p(0.50, 0.86),
            ];
            painter.add(egui::Shape::line(arc_left, thin_stroke));
            let arc_right = vec![
                p(0.50, 0.14),
                p(0.68, 0.30),
                p(0.72, 0.50),
                p(0.68, 0.70),
                p(0.50, 0.86),
            ];
            painter.add(egui::Shape::line(arc_right, thin_stroke));
        }
        Icon::ArrowLeft => {
            painter.line_segment([p(0.74, 0.50), p(0.26, 0.50)], bold_stroke);
            painter.add(egui::Shape::line(
                vec![p(0.48, 0.26), p(0.26, 0.50), p(0.48, 0.74)],
                bold_stroke,
            ));
        }
        Icon::ArrowRight => {
            painter.line_segment([p(0.26, 0.50), p(0.74, 0.50)], bold_stroke);
            painter.add(egui::Shape::line(
                vec![p(0.52, 0.26), p(0.74, 0.50), p(0.52, 0.74)],
                bold_stroke,
            ));
        }
        Icon::Home => {
            let roof = vec![p(0.16, 0.46), p(0.50, 0.18), p(0.84, 0.46)];
            painter.add(egui::Shape::line(roof, bold_stroke));
            let body = vec![p(0.24, 0.46), p(0.24, 0.82), p(0.76, 0.82), p(0.76, 0.46)];
            painter.add(egui::Shape::closed_line(body, stroke));
            let door = vec![p(0.42, 0.82), p(0.42, 0.58), p(0.58, 0.58), p(0.58, 0.82)];
            painter.add(egui::Shape::line(door, thin_stroke));
        }
        Icon::Refresh => {
            let center = p(0.50, 0.50);
            let r = w * 0.28;
            let start_ang = 0.35 * PI;
            let end_ang = 1.95 * PI;
            let steps = 12;
            let mut arc_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = start_ang + (end_ang - start_ang) * (i as f32 / steps as f32);
                arc_pts.push(Pos2::new(center.x + r * a.cos(), center.y + r * a.sin()));
            }
            painter.add(egui::Shape::line(arc_pts, bold_stroke));
            let tip = Pos2::new(center.x + r * end_ang.cos(), center.y + r * end_ang.sin());
            let head = vec![
                Pos2::new(tip.x - w * 0.14, tip.y - h * 0.04),
                tip,
                Pos2::new(tip.x - w * 0.02, tip.y - h * 0.15),
            ];
            painter.add(egui::Shape::line(head, bold_stroke));
        }
        Icon::Download => {
            painter.line_segment([p(0.50, 0.16), p(0.50, 0.58)], bold_stroke);
            let arrow = vec![p(0.32, 0.42), p(0.50, 0.60), p(0.68, 0.42)];
            painter.add(egui::Shape::line(arrow, bold_stroke));
            let tray = vec![p(0.18, 0.64), p(0.18, 0.82), p(0.82, 0.82), p(0.82, 0.64)];
            painter.add(egui::Shape::line(tray, bold_stroke));
        }
        Icon::Cross => {
            painter.line_segment([p(0.26, 0.26), p(0.74, 0.74)], bold_stroke);
            painter.line_segment([p(0.74, 0.26), p(0.26, 0.74)], bold_stroke);
        }
        Icon::Copy => {
            let back = Rect::from_min_max(p(0.32, 0.16), p(0.82, 0.68));
            painter.rect_stroke(back, 1.5, thin_stroke, StrokeKind::Inside);
            let front = Rect::from_min_max(p(0.18, 0.32), p(0.68, 0.84));
            painter.rect_filled(front, 1.5, color.gamma_multiply(0.15));
            painter.rect_stroke(front, 1.5, stroke, StrokeKind::Inside);
        }
        Icon::Key => {
            let center = p(0.35, 0.50);
            let r = w * 0.16;
            painter.circle_stroke(center, r, stroke);
            painter.line_segment([p(0.51, 0.50), p(0.82, 0.50)], bold_stroke);
            painter.line_segment([p(0.68, 0.50), p(0.68, 0.68)], bold_stroke);
            painter.line_segment([p(0.80, 0.50), p(0.80, 0.64)], bold_stroke);
        }
        Icon::Bookmark => {
            let ribbon = vec![
                p(0.28, 0.16),
                p(0.72, 0.16),
                p(0.72, 0.84),
                p(0.50, 0.66),
                p(0.28, 0.84),
            ];
            painter.add(egui::Shape::closed_line(ribbon, stroke));
        }
        Icon::Quote => {
            let q1 = vec![p(0.26, 0.44), p(0.36, 0.44), p(0.30, 0.66)];
            painter.add(egui::Shape::line(q1, bold_stroke));
            painter.circle_filled(p(0.31, 0.44), 2.2, color);
            let q2 = vec![p(0.56, 0.44), p(0.66, 0.44), p(0.60, 0.66)];
            painter.add(egui::Shape::line(q2, bold_stroke));
            painter.circle_filled(p(0.61, 0.44), 2.2, color);
        }
        Icon::Edit => {
            let barrel = vec![p(0.30, 0.70), p(0.68, 0.32), p(0.78, 0.42), p(0.40, 0.80)];
            painter.add(egui::Shape::closed_line(barrel, stroke));
            let tip = vec![p(0.30, 0.70), p(0.22, 0.86), p(0.40, 0.80)];
            painter.add(egui::Shape::closed_line(tip, thin_stroke));
            painter.line_segment([p(0.20, 0.90), p(0.85, 0.90)], thin_stroke);
        }
        Icon::Headphones => {
            let arc = vec![
                p(0.24, 0.54),
                p(0.25, 0.38),
                p(0.34, 0.22),
                p(0.50, 0.16),
                p(0.66, 0.22),
                p(0.75, 0.38),
                p(0.76, 0.54),
            ];
            painter.add(egui::Shape::line(arc, bold_stroke));
            let left_cup = Rect::from_min_max(p(0.16, 0.48), p(0.28, 0.82));
            painter.rect_stroke(left_cup, 2.5, stroke, StrokeKind::Inside);
            painter.rect_filled(left_cup.shrink(1.0), 2.0, color.gamma_multiply(0.2));
            let right_cup = Rect::from_min_max(p(0.72, 0.48), p(0.84, 0.82));
            painter.rect_stroke(right_cup, 2.5, stroke, StrokeKind::Inside);
            painter.rect_filled(right_cup.shrink(1.0), 2.0, color.gamma_multiply(0.2));
        }
        Icon::Moon => {
            let pts = vec![
                p(0.58, 0.16),
                p(0.70, 0.24),
                p(0.78, 0.40),
                p(0.76, 0.60),
                p(0.66, 0.76),
                p(0.48, 0.84),
                p(0.28, 0.80),
                p(0.44, 0.70),
                p(0.54, 0.54),
                p(0.54, 0.36),
                p(0.46, 0.22),
            ];
            painter.add(egui::Shape::closed_line(pts, bold_stroke));
        }
        Icon::Crop => {
            painter.line_segment([p(0.18, 0.34), p(0.76, 0.34)], bold_stroke);
            painter.line_segment([p(0.34, 0.18), p(0.34, 0.76)], bold_stroke);
            painter.line_segment([p(0.24, 0.66), p(0.82, 0.66)], bold_stroke);
            painter.line_segment([p(0.66, 0.24), p(0.66, 0.82)], bold_stroke);
        }
    }
}

pub struct IconButton<'a> {
    pub icon: Icon,
    pub text: &'a str,
    pub font_size: Option<f32>,
    pub min_size: Option<Vec2>,
    pub selected: bool,
    pub center_content: bool,
    pub color: Option<Color32>,
    pub fill: Option<Color32>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon, text: &'a str) -> Self {
        Self {
            icon,
            text,
            font_size: None,
            min_size: None,
            selected: false,
            center_content: false,
            color: None,
            fill: None,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = Some(size);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn center(mut self, center: bool) -> Self {
        self.center_content = center;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    #[allow(dead_code)]
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }
}

impl<'a> Widget for IconButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let padding = ui.spacing().button_padding;
        let icon_size = self.font_size.unwrap_or(15.0);
        let has_text = !self.text.is_empty();
        let gap = if has_text { 6.0 } else { 0.0 };

        let font_id = match self.font_size {
            Some(sz) => egui::FontId::proportional(sz),
            None => egui::TextStyle::Button.resolve(ui.style()),
        };
        let initial_text_color = self.color.unwrap_or_else(|| {
            ui.visuals()
                .override_text_color
                .unwrap_or_else(|| ui.visuals().text_color())
        });
        let galley =
            ui.painter()
                .layout_no_wrap(self.text.to_string(), font_id.clone(), initial_text_color);
        let text_size = if has_text { galley.size() } else { Vec2::ZERO };

        let mut desired_size = Vec2::new(
            padding.x * 2.0 + icon_size + gap + text_size.x,
            (text_size.y.max(icon_size) + padding.y * 2.0).max(ui.spacing().interact_size.y),
        );

        if let Some(min_sz) = self.min_size {
            desired_size.x = desired_size.x.max(min_sz.x);
            desired_size.y = desired_size.y.max(min_sz.y);
        }

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = if self.selected {
                ui.style().interact_selectable(&response, true)
            } else {
                *ui.style().interact(&response)
            };

            let item_color = self.color.unwrap_or_else(|| visuals.text_color());
            let galley = if item_color != initial_text_color && has_text {
                ui.painter()
                    .layout_no_wrap(self.text.to_string(), font_id, item_color)
            } else {
                galley
            };

            let bg_fill = self.fill.unwrap_or(visuals.bg_fill);

            ui.painter().rect(
                rect,
                visuals.corner_radius,
                bg_fill,
                visuals.bg_stroke,
                StrokeKind::Inside,
            );

            let content_w = if has_text {
                icon_size + gap + text_size.x
            } else {
                icon_size
            };
            let start_x = if self.center_content || self.min_size.is_some() || !has_text {
                rect.min.x + (rect.width() - content_w).max(0.0) * 0.5
            } else {
                rect.min.x + padding.x
            };

            let icon_rect = Rect::from_center_size(
                Pos2::new(start_x + icon_size * 0.5, rect.center().y),
                Vec2::splat(icon_size),
            );
            paint_icon(ui.painter(), self.icon, icon_rect, item_color);

            if has_text {
                let text_pos = Pos2::new(
                    start_x + icon_size + gap,
                    rect.center().y - text_size.y * 0.5,
                );
                ui.painter().galley(text_pos, galley, item_color);
            }
        }

        response
    }
}

pub struct IconLabel<'a> {
    pub icon: Icon,
    pub text: &'a str,
    pub font_size: f32,
    pub color: Option<Color32>,
    pub strong: bool,
}

impl<'a> IconLabel<'a> {
    pub fn new(icon: Icon, text: &'a str) -> Self {
        Self {
            icon,
            text,
            font_size: 14.0,
            color: None,
            strong: false,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }
}

impl<'a> Widget for IconLabel<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_color = self.color.unwrap_or_else(|| ui.visuals().text_color());
        let icon_size = self.font_size;
        let has_text = !self.text.is_empty();
        let gap = if has_text { 6.0 } else { 0.0 };

        let font_id = if self.strong {
            egui::FontId::new(
                self.font_size,
                egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
            )
        } else {
            egui::FontId::proportional(self.font_size)
        };

        let galley = if has_text {
            Some(
                ui.painter()
                    .layout_no_wrap(self.text.to_string(), font_id, text_color),
            )
        } else {
            None
        };
        let text_size = galley.as_ref().map(|g| g.size()).unwrap_or(Vec2::ZERO);

        let desired_size = if has_text {
            Vec2::new(icon_size + gap + text_size.x, text_size.y.max(icon_size))
        } else {
            Vec2::splat(icon_size)
        };
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let icon_rect = Rect::from_center_size(
                Pos2::new(rect.min.x + icon_size * 0.5, rect.center().y),
                Vec2::splat(icon_size),
            );
            paint_icon(ui.painter(), self.icon, icon_rect, text_color);

            if let Some(g) = galley {
                let text_pos = Pos2::new(
                    rect.min.x + icon_size + gap,
                    rect.center().y - text_size.y * 0.5,
                );
                ui.painter().galley(text_pos, g, text_color);
            }
        }

        response
    }
}

pub fn icon_button<'a>(ui: &mut Ui, icon: Icon, text: &'a str) -> Response {
    ui.add(IconButton::new(icon, text))
}

pub fn selectable_icon_button<'a>(
    ui: &mut Ui,
    selected: bool,
    icon: Icon,
    text: &'a str,
) -> Response {
    ui.add(
        IconButton::new(icon, text)
            .selected(selected)
            .center(true)
            .min_size(Vec2::new(115.0, 32.0)),
    )
}
