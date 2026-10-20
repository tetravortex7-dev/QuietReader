use crate::document::{DocumentError, PdfDocument};
use crate::test::util::unique_test_dir;
use crate::*;
use std::path::Path;

#[test]
fn test_corrupted_and_truncated_pdf_handling() {
    let dummy_path = Path::new("test.pdf");

    let res_empty = PdfDocument::from_bytes(Vec::new(), dummy_path);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_short = PdfDocument::from_bytes(vec![b'%', b'P'], dummy_path);
    assert!(matches!(res_short, Err(DocumentError::Corrupted(_))));

    let res_garbage = PdfDocument::from_bytes(b"THIS_IS_DEFINITELY_NOT_A_PDF".to_vec(), dummy_path);
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_header_only = PdfDocument::from_bytes(b"%PDF-1.7\n%%EOF".to_vec(), dummy_path);
    assert!(matches!(res_header_only, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_pdf_reading_order_two_columns_and_gutter() {
    use crate::document::pdf::text::{GlyphEntry, group_and_sort_glyphs};
    let mut glyphs = Vec::new();
    for (i, c) in "Left1".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 300.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right1".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 300.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Left2".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 350.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right2".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 350.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Left3".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 400.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right3".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 400.0,
            font_size: 14.0,
            width: 10.0,
        });
    }

    let spans = group_and_sort_glyphs(glyphs, 800.0, 1000.0);
    assert_eq!(spans.len(), 6);
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left1", "Left2", "Left3", "Right1", "Right2", "Right3"]
    );
}

#[test]
fn test_pdf_outline_resolve_dest_clamping() {
    use hayro::hayro_syntax::object::{Array, FromBytes};
    use std::collections::HashMap;

    let dest_data = b"[ 5 /XYZ 0 792 1.0 ]";
    if let Some(arr) = Array::from_bytes(dest_data) {
        let page_map = HashMap::new();
        let page_count = 3;
        let resolved =
            crate::document::pdf::outline::resolve_dest_array(&arr, &page_map, page_count);
        assert_eq!(resolved, Some(2));
    }
}

#[test]
fn test_pdf_document_parsing_and_rasterization() {
    let minimal_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Resources <<>> >>
endobj
xref
0 4
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
trailer
<< /Size 4 /Root 1 0 R >>
startxref
206
%%EOF";

    let temp_dir = unique_test_dir("pdf");
    let pdf_path = temp_dir.join("test_sample.pdf");
    std::fs::write(&pdf_path, minimal_pdf).unwrap();

    let doc = document::open_document(&pdf_path).expect("Должен успешно открыть валидный PDF");
    assert_eq!(doc.chapters().len(), 1);
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Страница 1");

    let comic = doc
        .page_content()
        .expect("PDF должен реализовывать ComicDocumentTrait для страничного отображения");
    assert_eq!(comic.page_count(), 1);
    assert!(!comic.is_manga());

    let page_img = comic
        .load_page_image(0)
        .expect("Страница PDF должна успешно растеризоваться");
    assert!(page_img.width > 0);
    assert!(page_img.height > 0);
    assert_eq!(
        page_img.rgba.len(),
        (page_img.width * page_img.height * 4) as usize
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_pdf_transitive_glyph_sorting_and_line_clustering() {
    use crate::document::pdf::text::{GlyphEntry, group_and_sort_glyphs};

    let glyphs = vec![
        GlyphEntry {
            c: 'l',
            x: 20.0,
            y: 10.2,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'H',
            x: 10.0,
            y: 10.0,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'e',
            x: 15.0,
            y: 9.8,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'o',
            x: 25.0,
            y: 10.1,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'W',
            x: 10.0,
            y: 30.0,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'o',
            x: 15.0,
            y: 30.2,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'r',
            x: 20.0,
            y: 29.9,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'l',
            x: 25.0,
            y: 30.0,
            font_size: 10.0,
            width: 5.0,
        },
        GlyphEntry {
            c: 'd',
            x: 30.0,
            y: 30.1,
            font_size: 10.0,
            width: 5.0,
        },
    ];

    let spans = group_and_sort_glyphs(glyphs, 200.0, 200.0);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "Helo");
    assert_eq!(spans[1].text, "World");
}
