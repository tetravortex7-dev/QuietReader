use super::{UiLanguage, fill, format::format_number};

pub fn decode_document_error_detail(raw: &str, lang: UiLanguage) -> Option<String> {
    let (code, args) = crate::document::errors::decode(raw)?;
    let t = lang.t();
    let e = &t.extra;
    let a = |i: usize| -> &str { args.get(i).copied().unwrap_or("") };
    let mb = |s: &str| format!("{s} {}", e.megabytes_short);
    let fmt_num = |s: &str| {
        s.parse::<usize>()
            .map(|n| format_number(n, lang))
            .unwrap_or_else(|_| s.to_string())
    };
    let bytes = |s: &str| format!("{} {}", fmt_num(s), e.bytes_short);

    let msg = match code {
        "err:io_open" => fill(e.err_io_open, &[("{target}", a(0)), ("{error}", a(1))]),
        "err:io_read" => fill(e.err_io_read, &[("{target}", a(0)), ("{error}", a(1))]),
        "err:io_meta" => fill(e.err_io_metadata, &[("{fmt}", a(0)), ("{error}", a(1))]),
        "err:not_found" => fill(e.err_io_not_found, &[("{path}", a(0))]),
        "err:arch_io" => fill(e.err_io_archive_read, &[("{error}", a(0))]),
        "err:cache_dir" => fill(e.err_cache_dir_create, &[("{error}", a(0))]),
        "err:cache_file" => fill(e.err_cache_file_create, &[("{error}", a(0))]),
        "err:cache_perm" => fill(e.err_cache_permissions, &[("{error}", a(0))]),
        "err:cache_save" => fill(e.err_cache_save_page, &[("{error}", a(0))]),
        "err:file_limit" => fill(e.err_file_too_large, &[("{fmt}", a(0))]),
        "err:file_limit_mb" => fill(
            e.err_file_too_large_limit,
            &[("{fmt}", a(0)), ("{limit}", &mb(a(1)))],
        ),
        "err:data_limit" => fill(e.err_data_too_large, &[("{fmt}", a(0))]),
        "err:data_limit_mb" => fill(
            e.err_data_too_large_limit,
            &[("{fmt}", a(0)), ("{limit}", &mb(a(1)))],
        ),
        "err:decomp_limit_mb" => fill(
            e.err_decompressed_too_large,
            &[("{fmt}", a(0)), ("{limit}", &mb(a(1)))],
        ),
        "err:arch_parse" => fill(e.err_archive_parse, &[("{fmt}", a(0)), ("{error}", a(1))]),
        "err:arch_read" => fill(e.err_archive_read, &[("{fmt}", a(0)), ("{error}", a(1))]),
        "err:arch_many" => fill(
            e.err_archive_too_many_files,
            &[("{fmt}", a(0)), ("{limit}", &fmt_num(a(1)))],
        ),
        "err:entry_limit" => fill(e.err_archive_entry_too_large, &[("{entry}", a(0))]),
        "err:entry_limit_mb" => fill(
            e.err_archive_entry_too_large_limit,
            &[("{entry}", a(0)), ("{limit}", &mb(a(1)))],
        ),
        "err:entry_limit_bytes" => fill(
            e.err_archive_entry_too_large_bytes,
            &[("{limit}", &bytes(a(1)))],
        ),
        "err:total_limit" => fill(e.err_archive_total_too_large, &[("{fmt}", a(0))]),
        "err:total_limit_mb" => fill(
            e.err_archive_total_too_large_fmt_limit,
            &[("{fmt}", a(0)), ("{limit}", &mb(a(1)))],
        ),
        "err:total_limit_bytes" => fill(
            e.err_archive_total_too_large_limit,
            &[("{limit}", &bytes(a(0)))],
        ),
        "err:arch_unsafe" => fill(
            e.err_archive_unsafe_path,
            &[("{fmt}", a(0)), ("{entry}", a(1))],
        ),
        "err:unsafe_path" => e.err_unsafe_path.to_string(),
        "err:entry_missing" => fill(
            e.err_archive_entry_not_found,
            &[("{fmt}", a(0)), ("{entry}", a(1))],
        ),
        "err:entry_err" => fill(
            e.err_archive_entry_error,
            &[("{entry}", a(0)), ("{error}", a(1))],
        ),
        "err:entry_read" => fill(
            e.err_archive_entry_read,
            &[("{entry}", a(0)), ("{error}", a(1))],
        ),
        "err:cache_unavail" => fill(e.err_archive_cache_unavailable, &[("{fmt}", a(0))]),
        "err:fb2_zip_missing" => e.err_fb2_missing_in_zip.to_string(),
        "err:fb2_no_root" => e.err_fb2_missing_root.to_string(),
        "err:fb2_extract" => e.err_fb2_extract_failed.to_string(),
        "err:epub_no_opf" => e.err_epub_no_rootfile.to_string(),
        "err:epub_opf" => fill(e.err_epub_parse_opf, &[("{error}", a(0))]),
        "err:epub_container" => fill(e.err_epub_parse_container, &[("{error}", a(0))]),
        "err:epub_empty_spine" => e.err_epub_empty_spine.to_string(),
        "err:comic_xml" => fill(e.err_comic_xml_parse, &[("{error}", a(0))]),
        "err:comic_unsup" => fill(e.err_comic_unsupported_format, &[("{ext}", a(0))]),
        "err:comic_no_imgs" => e.err_comic_no_images.to_string(),
        "err:unsup_fmt" => fill(e.err_unsupported_format, &[("{path}", a(0))]),
        "err:short_file" => fill(e.err_file_too_short, &[("{fmt}", a(0))]),
        "err:bad_header" => fill(e.err_invalid_header, &[("{fmt}", a(0)), ("{detail}", a(1))]),
        "err:file_empty" => fill(e.err_file_empty, &[("{fmt}", a(0))]),
        "err:bad_sector" => fill(e.err_invalid_sector_size, &[("{fmt}", a(0))]),
        "err:parse_failed" => fill(e.err_parse_failed, &[("{fmt}", a(0)), ("{error}", a(1))]),
        "err:parse_corrupt" => fill(e.err_parse_corrupted, &[("{fmt}", a(0))]),
        "err:decoder_panic" => fill(e.err_decoder_panic, &[("{fmt}", a(0))]),
        "err:no_pages" => fill(e.err_no_pages, &[("{fmt}", a(0))]),
        "err:no_read_text" => fill(e.err_no_readable_text, &[("{fmt}", a(0))]),
        "err:empty_doc" => fill(e.err_empty_document, &[("{fmt}", a(0))]),
        "err:binary_file" => e.err_binary_file.to_string(),
        "err:extract_chaps" => e.err_extract_chapters_failed.to_string(),
        "err:page_oob" => fill(e.err_page_out_of_bounds, &[("{page}", a(0))]),
        "err:page_missing" => fill(e.err_page_not_found, &[("{page}", a(0))]),
        "err:page_get" => fill(e.err_page_get_error, &[("{page}", a(0)), ("{error}", a(1))]),
        "err:page_bad_dims" => fill(e.err_page_invalid_dims, &[("{page}", a(0))]),
        "err:page_zero" => fill(e.err_page_zero_size, &[("{page}", a(0))]),
        "err:page_render" => fill(
            e.err_page_render_error,
            &[("{fmt}", a(0)), ("{page}", a(1)), ("{error}", a(2))],
        ),
        "err:page_buf" => fill(
            e.err_page_buffer_mismatch,
            &[("{page}", a(0)), ("{expected}", a(1)), ("{actual}", a(2))],
        ),
        "err:page_panic" => fill(e.err_page_render_panic, &[("{page}", a(0))]),
        "err:img_missing" => e.err_image_not_found.to_string(),
        "err:img_empty" => e.err_image_empty.to_string(),
        "err:img_fmt" => fill(e.err_image_format_error, &[("{error}", a(0))]),
        "err:img_zero" => e.err_image_zero_size.to_string(),
        "err:img_limit" => fill(
            e.err_image_too_large,
            &[("{width}", a(0)), ("{height}", a(1))],
        ),
        "err:img_decode" => fill(e.err_image_decode_failed, &[("{error}", a(0))]),
        _ => return None,
    };
    Some(msg)
}
