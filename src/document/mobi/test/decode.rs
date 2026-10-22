use crate::document::mobi::decode::*;

#[test]
fn test_decompress_palmdoc_plain() {
    let plain = b"Hello world!";
    let decompressed = decompress_palmdoc(plain).expect("decompress");
    assert_eq!(decompressed, plain);
}

#[test]
fn test_decompress_palmdoc_repeat() {
    let mut data = Vec::new();
    data.extend_from_slice(b"abc");
    let b1 = 0x80 | (3 >> 5);
    let b2 = ((3 & 0x1F) << 3) | 0;
    data.push(b1 as u8);
    data.push(b2 as u8);

    let decompressed = decompress_palmdoc(&data).expect("decompress");
    assert_eq!(decompressed, b"abcabc");
}

#[test]
fn test_cp1252_mojibake_detection_and_repair() {
    let original = "Привет, мир! Это тестовое сообщение.";
    let (bytes_1251, _, _) = encoding_rs::WINDOWS_1251.encode(original);
    let (mojibake, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes_1251);

    assert!(is_cp1252_mojibake(&mojibake));
    let repaired = repair_cp1252_mojibake(&mojibake);
    assert_eq!(repaired, original);
}

#[test]
fn test_decode_mobi_content_cyrillic_cp1251() {
    let russian_html = "<html><body><h1>Глава первая</h1><p>Это тест декодирования русского текста.</p></body></html>";
    let (bytes_cp1251, _, _) = encoding_rs::WINDOWS_1251.encode(russian_html);
    let decoded = decode_mobi_content(&bytes_cp1251, 1252);

    assert!(decoded.contains("Глава первая"));
    assert!(decoded.contains("Это тест декодирования русского текста."));
    assert!(!decoded.contains('\u{FFFD}'));
}

#[test]
fn test_decode_metadata_string() {
    let title_ru = "Война и мир";
    let (title_bytes, _, _) = encoding_rs::WINDOWS_1251.encode(title_ru);
    let decoded_title = decode_metadata_string(&title_bytes, Some(encoding_rs::WINDOWS_1251), 1252);
    assert_eq!(decoded_title, "Война и мир");

    let author_ru = "Лев Толстой";
    let (author_bytes, _, _) = encoding_rs::WINDOWS_1251.encode(author_ru);
    let decoded_author = decode_metadata_string(&author_bytes, None, 1252);
    assert_eq!(decoded_author, "Лев Толстой");

    let utf8_title = "Преступление и наказание".as_bytes();
    let decoded_utf8 = decode_metadata_string(utf8_title, None, 1252);
    assert_eq!(decoded_utf8, "Преступление и наказание");
}
