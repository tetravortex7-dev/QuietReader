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

fn canonical_lang_alias(token: &str) -> Option<&'static str> {
    match token {
        "ru" | "rus" | "russian" | "русский" => Some("ru"),
        "en" | "eng" | "english" => Some("en"),
        "uk" | "ukr" | "ukrainian" | "українська" | "украинский" => Some("uk"),
        "de" | "ger" | "deu" | "german" | "deutsch" => Some("de"),
        "fr" | "fre" | "fra" | "french" | "français" | "francais" => Some("fr"),
        "es" | "spa" | "spanish" | "español" | "espanol" => Some("es"),
        "it" | "ita" | "italian" | "italiano" => Some("it"),
        "pt" | "por" | "portuguese" | "português" | "portugues" => Some("pt"),
        "tr" | "tur" | "turkish" | "türkçe" | "turkce" => Some("tr"),
        "zh" | "chi" | "cmn" | "zho" | "chinese" => Some("zh"),
        "ja" | "jpn" | "jap" | "japanese" => Some("ja"),
        "ar" | "ara" | "arabic" => Some("ar"),
        "id" | "ind" | "indonesian" => Some("id"),
        _ => None,
    }
}

pub fn voice_matches_lang(voice_lang: &str, book_lang: &str) -> bool {
    let voice_tokens: Vec<String> = voice_lang
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
        .collect();
    let book_tokens: Vec<String> = book_lang
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
        .collect();

    let (Some(voice_primary), Some(book_primary)) = (voice_tokens.first(), book_tokens.first())
    else {
        return false;
    };

    if let Some(book_canon) = canonical_lang_alias(book_primary) {
        if canonical_lang_alias(voice_primary) == Some(book_canon) {
            return true;
        }
        return voice_tokens
            .iter()
            .skip(1)
            .any(|tok| tok.chars().count() >= 3 && canonical_lang_alias(tok) == Some(book_canon));
    }

    voice_primary == book_primary
}
