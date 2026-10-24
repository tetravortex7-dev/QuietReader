use super::pdf_view::{PdfViewState, span_matches_search};
use crate::document::{PageLink, PageTextSpan};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Vec2};

impl PdfViewState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_page_with_selection(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        page_idx: usize,
        tex: &egui::TextureHandle,
        size: Vec2,
        rotation: i32,
        uv_rect: Rect,
        auto_crop: bool,
        selected_spans_collector: &mut Vec<String>,
        highlight_query: Option<&str>,
        links: &[PageLink],
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter();
        let rot_deg = rotation.rem_euclid(360);
        let u0 = uv_rect.min.x;
        let v0 = uv_rect.min.y;
        let u1 = uv_rect.max.x;
        let v1 = uv_rect.max.y;

        if rot_deg == 0 {
            painter.image(tex.id(), rect, uv_rect, Color32::WHITE);
        } else {
            let uvs = match rot_deg {
                90 => [
                    Pos2::new(u0, v1),
                    Pos2::new(u0, v0),
                    Pos2::new(u1, v0),
                    Pos2::new(u1, v1),
                ],
                180 => [
                    Pos2::new(u1, v1),
                    Pos2::new(u0, v1),
                    Pos2::new(u0, v0),
                    Pos2::new(u1, v0),
                ],
                270 => [
                    Pos2::new(u1, v0),
                    Pos2::new(u1, v1),
                    Pos2::new(u0, v1),
                    Pos2::new(u0, v0),
                ],
                _ => [
                    Pos2::new(u0, v0),
                    Pos2::new(u1, v0),
                    Pos2::new(u1, v1),
                    Pos2::new(u0, v1),
                ],
            };
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ];
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            for i in 0..4 {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: corners[i],
                    uv: uvs[i],
                    color: Color32::WHITE,
                });
            }
            painter.add(mesh);
        }

        let adjust_bbox = |bbox: [f32; 4]| -> Rect {
            let bbox = if auto_crop {
                let bw = uv_rect.width().max(0.001);
                let bh = uv_rect.height().max(0.001);
                [
                    (bbox[0] - u0) / bw,
                    (bbox[1] - v0) / bh,
                    (bbox[2] - u0) / bw,
                    (bbox[3] - v0) / bh,
                ]
            } else {
                bbox
            };
            super::paged_common::transform_span_bbox(bbox, rotation, rect)
        };
        let adjust_span = |span: &PageTextSpan| -> Rect { adjust_bbox(span.bbox) };

        if let Some(hover_pos) = response.hover_pos() {
            for link in links {
                let link_rect = adjust_bbox(link.bbox);
                if link_rect.contains(hover_pos) {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    painter.rect_filled(
                        link_rect,
                        CornerRadius::same(2),
                        Color32::from_rgba_unmultiplied(33, 150, 243, 35),
                    );
                    break;
                }
            }
        }

        if let Some(query) = highlight_query {
            let normalized = crate::core::normalize_search_text(query);
            if !normalized.is_empty()
                && let Some(spans) = self.page_text_spans.get(&page_idx)
            {
                for span in spans {
                    if span_matches_search(&span.text, &normalized) {
                        painter.rect_filled(
                            adjust_span(span),
                            CornerRadius::same(2),
                            Color32::from_rgba_unmultiplied(255, 193, 7, 90),
                        );
                    }
                }
            }
        }

        if let Some(hover_pos) = response.hover_pos()
            && let Some(spans) = self.page_text_spans.get(&page_idx)
        {
            for span in spans {
                let span_rect = adjust_span(span);
                if span_rect.contains(hover_pos) {
                    ctx.set_cursor_icon(egui::CursorIcon::Text);
                    break;
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos();
            self.selection_start = pos;
            self.selection_current = pos;
            self.selection_dirty = true;
        } else if response.dragged_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos();
            if self.selection_current != pos {
                self.selection_current = pos;
                self.selection_dirty = true;
            }
        } else if response.clicked_by(egui::PointerButton::Primary) {
            let target = response.interact_pointer_pos().and_then(|position| {
                links.iter().find_map(|link| {
                    adjust_bbox(link.bbox)
                        .contains(position)
                        .then_some(link.target_page)
                })
            });
            if let Some(target_page) = target {
                self.pending_target_page = Some(target_page);
            } else {
                self.selection_start = None;
                self.selection_current = None;
                self.selection_dirty = false;
                self.selected_text.clear();
            }
        }

        if let (Some(start), Some(curr)) = (self.selection_start, self.selection_current) {
            let sel_rect = Rect::from_two_pos(start, curr);
            if (sel_rect.width() > 3.0 || sel_rect.height() > 3.0)
                && rect.intersects(sel_rect)
                && let Some(spans) = self.page_text_spans.get(&page_idx)
            {
                for span in spans {
                    let span_rect = adjust_span(span);
                    if span_rect.intersects(sel_rect) {
                        painter.rect_filled(
                            span_rect,
                            CornerRadius::same(2),
                            Color32::from_rgba_unmultiplied(33, 150, 243, 90),
                        );
                        if self.selection_dirty {
                            selected_spans_collector.push(span.text.clone());
                        }
                    }
                }
            }
        }

        response
    }
}
