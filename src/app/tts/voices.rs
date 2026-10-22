use super::types::TtsVoiceInfo;

pub(super) fn default_edge_voices() -> Vec<TtsVoiceInfo> {
    const VOICES: &[(&str, &str, &str)] = &[
        ("ru-RU-SvetlanaNeural", "Светлана", "ru-RU"),
        ("ru-RU-DmitryNeural", "Дмитрий", "ru-RU"),
        ("en-US-JennyNeural", "Jenny", "en-US"),
        ("en-US-GuyNeural", "Guy", "en-US"),
        ("en-US-AriaNeural", "Aria", "en-US"),
        ("uk-UA-PolinaNeural", "Поліна", "uk-UA"),
        ("uk-UA-OstapNeural", "Остап", "uk-UA"),
        ("de-DE-KatjaNeural", "Katja", "de-DE"),
        ("de-DE-ConradNeural", "Conrad", "de-DE"),
        ("fr-FR-DeniseNeural", "Denise", "fr-FR"),
        ("fr-FR-HenriNeural", "Henri", "fr-FR"),
        ("es-ES-ElviraNeural", "Elvira", "es-ES"),
        ("es-ES-AlvaroNeural", "Alvaro", "es-ES"),
        ("it-IT-ElsaNeural", "Elsa", "it-IT"),
        ("it-IT-DiegoNeural", "Diego", "it-IT"),
        ("pt-BR-FranciscaNeural", "Francisca", "pt-BR"),
        ("pt-BR-AntonioNeural", "Antonio", "pt-BR"),
        ("tr-TR-EmelNeural", "Emel", "tr-TR"),
        ("tr-TR-AhmetNeural", "Ahmet", "tr-TR"),
        ("zh-CN-XiaoxiaoNeural", "Xiaoxiao", "zh-CN"),
        ("zh-CN-YunxiNeural", "Yunxi", "zh-CN"),
        ("ja-JP-NanamiNeural", "Nanami", "ja-JP"),
        ("ja-JP-KeitaNeural", "Keita", "ja-JP"),
        ("ar-SA-ZariyahNeural", "Zariyah", "ar-SA"),
        ("ar-SA-HamedNeural", "Hamed", "ar-SA"),
        ("id-ID-GadisNeural", "Gadis", "id-ID"),
        ("id-ID-ArdiNeural", "Ardi", "id-ID"),
    ];
    VOICES
        .iter()
        .map(|&(id, name, lang)| TtsVoiceInfo {
            id: id.to_string(),
            name: name.to_string(),
            language: lang.to_string(),
        })
        .collect()
}

pub(super) fn format_voice_name(name: &str) -> String {
    let s = name.trim();
    let s = s.strip_prefix("Microsoft ").unwrap_or(s);
    let s = s.strip_suffix(" Desktop").unwrap_or(s);
    let s = s.strip_suffix(" Mobile").unwrap_or(s);
    if let Some(idx) = s.find(" Online (Natural)") {
        return s[..idx].trim().to_string();
    }
    s.trim().to_string()
}

pub fn voice_matches_lang(voice_lang: &str, book_lang: &str) -> bool {
    let vl = voice_lang.to_ascii_lowercase();
    let bl = book_lang.to_ascii_lowercase();
    if vl.starts_with(&bl) || bl.starts_with(&vl) {
        return true;
    }
    match bl.as_str() {
        "ru" => vl.contains("ru") || vl.contains("rus"),
        "en" => vl.contains("en") || vl.contains("eng"),
        "uk" => vl.contains("uk") || vl.contains("ukr"),
        "de" => vl.contains("de") || vl.contains("ger") || vl.contains("deu"),
        "fr" => vl.contains("fr") || vl.contains("fre") || vl.contains("fra"),
        "es" => vl.contains("es") || vl.contains("spa"),
        "it" => vl.contains("it") || vl.contains("ita"),
        "pt" => vl.contains("pt") || vl.contains("por"),
        "tr" => vl.contains("tr") || vl.contains("tur"),
        "zh" => vl.contains("zh") || vl.contains("chi") || vl.contains("cmn") || vl.contains("zho"),
        "ja" => vl.contains("ja") || vl.contains("jpn") || vl.contains("jap"),
        "ar" => vl.contains("ar") || vl.contains("ara"),
        "id" => vl.contains("id") || vl.contains("ind"),
        _ => false,
    }
}
