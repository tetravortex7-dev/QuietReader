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
fn test_empty_pdf_file_returns_corrupted() {
    let dir = unique_test_dir("pdf_empty");
    let path = dir.join("empty.pdf");
    std::fs::write(&path, b"").unwrap();
    let result = PdfDocument::from_file(&path);
    assert!(matches!(result, Err(DocumentError::Corrupted(_))));
    let _ = std::fs::remove_dir_all(&dir);
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
fn test_pdf_internal_link_annotation() {
    use crate::document::Document;

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Annots [4 0 R] /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Annot /Subtype /Link /Rect [10 10 100 40] /A << /S /GoTo /D [6 0 R /Fit] >> >>"
            .to_string(),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Contents 7 0 R >>".to_string(),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref_start = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    let doc = PdfDocument::from_bytes(pdf, Path::new("links.pdf")).unwrap();
    let links = doc.page_content().unwrap().page_links(0);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_page, 1);
    assert!(links[0].bbox[2] > links[0].bbox[0]);
    assert!(links[0].bbox[3] > links[0].bbox[1]);
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
    assert_eq!(doc.toc()[0].title, crate::document::errors::tokens::page(1));

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

    let page_img_again = comic
        .load_page_image(0)
        .expect("Повторная страница PDF должна использовать кэш");
    assert_eq!(page_img_again.width, page_img.width);
    assert_eq!(page_img_again.height, page_img.height);
    assert_eq!(comic.extract_page_text(0), comic.extract_page_text(0));

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

#[test]
fn test_pdf_permissions_extraction() {
    use crate::document::pdf::extract_pdf_permissions;

    let unencrypted = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";
    let perms = extract_pdf_permissions(unencrypted);
    assert!(perms.can_print);
    assert!(perms.can_copy);

    let encrypted_no_print = b"4 0 obj\n<< /Filter /Standard /V 2 /R 3 /P -8 >>\nendobj\n";
    let perms_no_print = extract_pdf_permissions(encrypted_no_print);
    assert!(!perms_no_print.can_print);
    assert!(perms_no_print.can_copy);

    let encrypted_no_copy = b"4 0 obj\n<< /Filter /Standard /V 2 /R 3 /P -20 >>\nendobj\n";
    let perms_no_copy = extract_pdf_permissions(encrypted_no_copy);
    assert!(perms_no_copy.can_print);
    assert!(!perms_no_copy.can_copy);

    let padding = "0".repeat(700);
    let long_encrypt_pdf = format!(
        "%PDF-1.7\n\
        1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
        4 0 obj\n<< /P -20 /O <{padding}> /U <{padding}> /V 5 /R 6 /Filter /Standard >>\nendobj\n\
        5 0 obj\n<< /Filter /FlateDecode /P -4 /Length 0 >>\nstream\nendstream\nendobj\n\
        trailer\n<< /Size 6 /Root 1 0 R /Encrypt 4 0 R >>\n%%EOF\n"
    );
    let long_perms = extract_pdf_permissions(long_encrypt_pdf.as_bytes());
    assert!(long_perms.can_print);
    assert!(!long_perms.can_copy);

    let unencrypted_with_decoy = b"%PDF-1.4\n\
        1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
        3 0 obj\n<< /Filter /FlateDecode /P -20 /Length 0 >>\nstream\nendstream\nendobj\n\
        trailer\n<< /Size 4 /Root 1 0 R >>\n%%EOF\n";
    let decoy_perms = extract_pdf_permissions(unencrypted_with_decoy);
    assert!(decoy_perms.can_print);
    assert!(decoy_perms.can_copy);
}

#[test]
fn test_pdf_from_bytes_with_password_api() {
    let dummy_path = Path::new("dummy.pdf");
    let result_none = PdfDocument::from_bytes_with_password(Vec::new(), dummy_path, None);
    assert!(matches!(result_none, Err(DocumentError::Corrupted(_))));

    let result_with_pass =
        PdfDocument::from_bytes_with_password(Vec::new(), dummy_path, Some("secret"));
    assert!(matches!(result_with_pass, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_pdf_names_dests_tree_in_outline_and_links() {
    use crate::document::Document;

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 8 0 R >> /Outlines 10 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Annots [6 0 R] /Contents 7 0 R >>"
            .to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Contents 7 0 R >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Contents 7 0 R >>".to_string(),
        "<< /Type /Annot /Subtype /Link /Rect [10 10 100 40] /Dest (sec_three) >>".to_string(),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
        "<< /Kids [9 0 R] >>".to_string(),
        "<< /Kids [8 0 R] /Names [(sec_two) [4 0 R /Fit] (sec_three) << /D [5 0 R /XYZ 0 250 0] >>] >>"
            .to_string(),
        "<< /Type /Outlines /First 11 0 R /Last 12 0 R /Count 2 >>".to_string(),
        "<< /Title (Part Two) /Parent 10 0 R /Next 12 0 R /Dest (sec_two) >>".to_string(),
        "<< /Title (Part Three) /Parent 10 0 R /Prev 11 0 R /A << /S /GoTo /D /sec_three >> >>"
            .to_string(),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref_start = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    let doc = PdfDocument::from_bytes(pdf, Path::new("names_dests.pdf")).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Part Two");
    assert_eq!(toc[0].chapter_index, 1);
    assert_eq!(toc[1].title, "Part Three");
    assert_eq!(toc[1].chapter_index, 2);

    let links = doc.page_content().unwrap().page_links(0);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_page, 2);
}
