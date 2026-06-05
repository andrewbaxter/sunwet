use {
    crate::log::Log,
    flowcontrol::shed,
    gloo::utils::window,
    serde::{
        Deserialize,
        Serialize,
    },
    std::rc::Rc,
    wasm_bindgen::{
        JsValue,
        UnwrapThrowExt,
    },
};

macro_rules! define_lang {
    ($($variant:ident),* $(,)?) => {
        #[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum Lang {
            $($variant),*
        }

        impl Lang {
            pub fn all() -> &'static [Lang] {
                &[$(Lang::$variant),*]
            }
        }
    };
}

define_lang! {
    Afrikaans, Albanian, Amharic, Arabic, Armenian, Azerbaijani, Basque, Belarusian, Bengali,
    Bosnian, Bulgarian, Burmese, Catalan, Chinese, Croatian, Czech, Danish, Dutch, English,
    Esperanto, Estonian, Finnish, French, Galician, Georgian, German, Greek, Gujarati, Hausa,
    Hebrew, Hindi, Hungarian, Icelandic, Igbo, Indonesian, Irish, Italian, Japanese, Javanese,
    Kannada, Kazakh, Khmer, Korean, Kurdish, Kyrgyz, Lao, Latin, Latvian, Lithuanian,
    Luxembourgish, Macedonian, Malay, Malayalam, Maltese, Maori, Marathi, Mongolian, Nepali,
    Norwegian, Pashto, Persian, Polish, Portuguese, Punjabi, Romanian, Russian, Serbian, Sinhala,
    Slovak, Slovenian, Somali, Spanish, Sundanese, Swahili, Swedish, Tagalog, Tajik, Tamil,
    Telugu, Thai, Turkish, Turkmen, Ukrainian, Urdu, Uzbek, Vietnamese, Welsh, Xhosa, Yoruba,
    Zulu,
}

impl Lang {
    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::Afrikaans => "Afrikaans",
            Lang::Albanian => "Albanian",
            Lang::Amharic => "Amharic",
            Lang::Arabic => "Arabic",
            Lang::Armenian => "Armenian",
            Lang::Azerbaijani => "Azerbaijani",
            Lang::Basque => "Basque",
            Lang::Belarusian => "Belarusian",
            Lang::Bengali => "Bengali",
            Lang::Bosnian => "Bosnian",
            Lang::Bulgarian => "Bulgarian",
            Lang::Burmese => "Burmese",
            Lang::Catalan => "Catalan",
            Lang::Chinese => "Chinese",
            Lang::Croatian => "Croatian",
            Lang::Czech => "Czech",
            Lang::Danish => "Danish",
            Lang::Dutch => "Dutch",
            Lang::English => "English",
            Lang::Esperanto => "Esperanto",
            Lang::Estonian => "Estonian",
            Lang::Finnish => "Finnish",
            Lang::French => "French",
            Lang::Galician => "Galician",
            Lang::Georgian => "Georgian",
            Lang::German => "German",
            Lang::Greek => "Greek",
            Lang::Gujarati => "Gujarati",
            Lang::Hausa => "Hausa",
            Lang::Hebrew => "Hebrew",
            Lang::Hindi => "Hindi",
            Lang::Hungarian => "Hungarian",
            Lang::Icelandic => "Icelandic",
            Lang::Igbo => "Igbo",
            Lang::Indonesian => "Indonesian",
            Lang::Irish => "Irish",
            Lang::Italian => "Italian",
            Lang::Japanese => "Japanese",
            Lang::Javanese => "Javanese",
            Lang::Kannada => "Kannada",
            Lang::Kazakh => "Kazakh",
            Lang::Khmer => "Khmer",
            Lang::Korean => "Korean",
            Lang::Kurdish => "Kurdish",
            Lang::Kyrgyz => "Kyrgyz",
            Lang::Lao => "Lao",
            Lang::Latin => "Latin",
            Lang::Latvian => "Latvian",
            Lang::Lithuanian => "Lithuanian",
            Lang::Luxembourgish => "Luxembourgish",
            Lang::Macedonian => "Macedonian",
            Lang::Malay => "Malay",
            Lang::Malayalam => "Malayalam",
            Lang::Maltese => "Maltese",
            Lang::Maori => "Māori",
            Lang::Marathi => "Marathi",
            Lang::Mongolian => "Mongolian",
            Lang::Nepali => "Nepali",
            Lang::Norwegian => "Norwegian",
            Lang::Pashto => "Pashto",
            Lang::Persian => "Persian",
            Lang::Polish => "Polish",
            Lang::Portuguese => "Portuguese",
            Lang::Punjabi => "Punjabi",
            Lang::Romanian => "Romanian",
            Lang::Russian => "Russian",
            Lang::Serbian => "Serbian",
            Lang::Sinhala => "Sinhala",
            Lang::Slovak => "Slovak",
            Lang::Slovenian => "Slovenian",
            Lang::Somali => "Somali",
            Lang::Spanish => "Spanish",
            Lang::Sundanese => "Sundanese",
            Lang::Swahili => "Swahili",
            Lang::Swedish => "Swedish",
            Lang::Tagalog => "Tagalog",
            Lang::Tajik => "Tajik",
            Lang::Tamil => "Tamil",
            Lang::Telugu => "Telugu",
            Lang::Thai => "Thai",
            Lang::Turkish => "Turkish",
            Lang::Turkmen => "Turkmen",
            Lang::Ukrainian => "Ukrainian",
            Lang::Urdu => "Urdu",
            Lang::Uzbek => "Uzbek",
            Lang::Vietnamese => "Vietnamese",
            Lang::Welsh => "Welsh",
            Lang::Xhosa => "Xhosa",
            Lang::Yoruba => "Yoruba",
            Lang::Zulu => "Zulu",
        }
    }

    pub fn iso639_3(&self) -> &'static str {
        match self {
            Lang::Afrikaans => "afr",
            Lang::Albanian => "sqi",
            Lang::Amharic => "amh",
            Lang::Arabic => "ara",
            Lang::Armenian => "hye",
            Lang::Azerbaijani => "aze",
            Lang::Basque => "eus",
            Lang::Belarusian => "bel",
            Lang::Bengali => "ben",
            Lang::Bosnian => "bos",
            Lang::Bulgarian => "bul",
            Lang::Burmese => "mya",
            Lang::Catalan => "cat",
            Lang::Chinese => "zho",
            Lang::Croatian => "hrv",
            Lang::Czech => "ces",
            Lang::Danish => "dan",
            Lang::Dutch => "nld",
            Lang::English => "eng",
            Lang::Esperanto => "epo",
            Lang::Estonian => "est",
            Lang::Finnish => "fin",
            Lang::French => "fra",
            Lang::Galician => "glg",
            Lang::Georgian => "kat",
            Lang::German => "deu",
            Lang::Greek => "ell",
            Lang::Gujarati => "guj",
            Lang::Hausa => "hau",
            Lang::Hebrew => "heb",
            Lang::Hindi => "hin",
            Lang::Hungarian => "hun",
            Lang::Icelandic => "isl",
            Lang::Igbo => "ibo",
            Lang::Indonesian => "ind",
            Lang::Irish => "gle",
            Lang::Italian => "ita",
            Lang::Japanese => "jpn",
            Lang::Javanese => "jav",
            Lang::Kannada => "kan",
            Lang::Kazakh => "kaz",
            Lang::Khmer => "khm",
            Lang::Korean => "kor",
            Lang::Kurdish => "kur",
            Lang::Kyrgyz => "kir",
            Lang::Lao => "lao",
            Lang::Latin => "lat",
            Lang::Latvian => "lav",
            Lang::Lithuanian => "lit",
            Lang::Luxembourgish => "ltz",
            Lang::Macedonian => "mkd",
            Lang::Malay => "msa",
            Lang::Malayalam => "mal",
            Lang::Maltese => "mlt",
            Lang::Maori => "mri",
            Lang::Marathi => "mar",
            Lang::Mongolian => "mon",
            Lang::Nepali => "nep",
            Lang::Norwegian => "nor",
            Lang::Pashto => "pus",
            Lang::Persian => "fas",
            Lang::Polish => "pol",
            Lang::Portuguese => "por",
            Lang::Punjabi => "pan",
            Lang::Romanian => "ron",
            Lang::Russian => "rus",
            Lang::Serbian => "srp",
            Lang::Sinhala => "sin",
            Lang::Slovak => "slk",
            Lang::Slovenian => "slv",
            Lang::Somali => "som",
            Lang::Spanish => "spa",
            Lang::Sundanese => "sun",
            Lang::Swahili => "swa",
            Lang::Swedish => "swe",
            Lang::Tagalog => "tgl",
            Lang::Tajik => "tgk",
            Lang::Tamil => "tam",
            Lang::Telugu => "tel",
            Lang::Thai => "tha",
            Lang::Turkish => "tur",
            Lang::Turkmen => "tuk",
            Lang::Ukrainian => "ukr",
            Lang::Urdu => "urd",
            Lang::Uzbek => "uzb",
            Lang::Vietnamese => "vie",
            Lang::Welsh => "cym",
            Lang::Xhosa => "xho",
            Lang::Yoruba => "yor",
            Lang::Zulu => "zul",
        }
    }

    pub fn from_nav_lang(nav_lang: &str) -> Option<Lang> {
        let short = if let Some((l, _)) = nav_lang.split_once("-") {
            l
        } else {
            nav_lang
        };
        let found = match short {
            "af" => Some(Lang::Afrikaans),
            "sq" => Some(Lang::Albanian),
            "am" => Some(Lang::Amharic),
            "ar" => Some(Lang::Arabic),
            "hy" => Some(Lang::Armenian),
            "az" => Some(Lang::Azerbaijani),
            "eu" => Some(Lang::Basque),
            "be" => Some(Lang::Belarusian),
            "bn" => Some(Lang::Bengali),
            "bs" => Some(Lang::Bosnian),
            "bg" => Some(Lang::Bulgarian),
            "my" => Some(Lang::Burmese),
            "ca" => Some(Lang::Catalan),
            "zh" => Some(Lang::Chinese),
            "hr" => Some(Lang::Croatian),
            "cs" => Some(Lang::Czech),
            "da" => Some(Lang::Danish),
            "nl" => Some(Lang::Dutch),
            "en" => Some(Lang::English),
            "eo" => Some(Lang::Esperanto),
            "et" => Some(Lang::Estonian),
            "fi" => Some(Lang::Finnish),
            "fr" => Some(Lang::French),
            "gl" => Some(Lang::Galician),
            "ka" => Some(Lang::Georgian),
            "de" => Some(Lang::German),
            "el" => Some(Lang::Greek),
            "gu" => Some(Lang::Gujarati),
            "ha" => Some(Lang::Hausa),
            "he" | "iw" => Some(Lang::Hebrew),
            "hi" => Some(Lang::Hindi),
            "hu" => Some(Lang::Hungarian),
            "is" => Some(Lang::Icelandic),
            "ig" => Some(Lang::Igbo),
            "id" => Some(Lang::Indonesian),
            "ga" => Some(Lang::Irish),
            "it" => Some(Lang::Italian),
            "ja" | "jp" => Some(Lang::Japanese),
            "jv" => Some(Lang::Javanese),
            "kn" => Some(Lang::Kannada),
            "kk" => Some(Lang::Kazakh),
            "km" => Some(Lang::Khmer),
            "ko" => Some(Lang::Korean),
            "ku" => Some(Lang::Kurdish),
            "ky" => Some(Lang::Kyrgyz),
            "lo" => Some(Lang::Lao),
            "la" => Some(Lang::Latin),
            "lv" => Some(Lang::Latvian),
            "lt" => Some(Lang::Lithuanian),
            "lb" => Some(Lang::Luxembourgish),
            "mk" => Some(Lang::Macedonian),
            "ms" => Some(Lang::Malay),
            "ml" => Some(Lang::Malayalam),
            "mt" => Some(Lang::Maltese),
            "mi" => Some(Lang::Maori),
            "mr" => Some(Lang::Marathi),
            "mn" => Some(Lang::Mongolian),
            "ne" => Some(Lang::Nepali),
            "no" | "nb" | "nn" => Some(Lang::Norwegian),
            "ps" => Some(Lang::Pashto),
            "fa" => Some(Lang::Persian),
            "pl" => Some(Lang::Polish),
            "pt" => Some(Lang::Portuguese),
            "pa" => Some(Lang::Punjabi),
            "ro" => Some(Lang::Romanian),
            "ru" => Some(Lang::Russian),
            "sr" => Some(Lang::Serbian),
            "si" => Some(Lang::Sinhala),
            "sk" => Some(Lang::Slovak),
            "sl" => Some(Lang::Slovenian),
            "so" => Some(Lang::Somali),
            "es" => Some(Lang::Spanish),
            "su" => Some(Lang::Sundanese),
            "sw" => Some(Lang::Swahili),
            "sv" => Some(Lang::Swedish),
            "tl" => Some(Lang::Tagalog),
            "tg" => Some(Lang::Tajik),
            "ta" => Some(Lang::Tamil),
            "te" => Some(Lang::Telugu),
            "th" => Some(Lang::Thai),
            "tr" => Some(Lang::Turkish),
            "tk" => Some(Lang::Turkmen),
            "uk" => Some(Lang::Ukrainian),
            "ur" => Some(Lang::Urdu),
            "uz" => Some(Lang::Uzbek),
            "vi" => Some(Lang::Vietnamese),
            "cy" => Some(Lang::Welsh),
            "xh" => Some(Lang::Xhosa),
            "yo" => Some(Lang::Yoruba),
            "zu" => Some(Lang::Zulu),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
        for lang in Lang::all() {
            if lang.iso639_3() == short {
                return Some(*lang);
            }
        }
        None
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// Since bug detection isn't a thing, or rather I don't want to deal with that
#[derive(Clone, PartialEq, Eq)]
pub enum Engine {
    IosSafari,
    Chrome,
}

#[derive(Clone)]
pub struct Env {
    // Ends with `/`
    pub base_url: String,
    pub engine: Option<Engine>,
    pub languages: Vec<Lang>,
    pub pwa: bool,
}

pub fn scan_env(log: &Rc<dyn Log>) -> Env {
    return Env {
        base_url: shed!{
            let loc = window().location();
            break format!(
                "{}{}/",
                loc.origin().unwrap_throw(),
                loc.pathname().unwrap_throw().rsplit_once("/").unwrap_throw().0
            );
        },
        engine: shed!{
            'found _;
            shed!{
                let user_agent = match window().navigator().user_agent() {
                    Ok(a) => a,
                    Err(e) => {
                        log.log_js("Error getting user agent to enable ios workarounds", &e);
                        break;
                    },
                };
                if user_agent.contains("iPad") || user_agent.contains("iPhone") || user_agent.contains("iPod") {
                    log.log("Detected mobile ios, activating webkit workarounds.");
                    break 'found Some(Engine::IosSafari);
                }
            }
            if js_sys::Reflect::has(&window(), &JsValue::from("chrome")).unwrap() {
                log.log("Detected chrome(ish), activating chrome workarounds.");
                break 'found Some(Engine::Chrome);
            }
            break None;
        },
        languages: shed!{
            let mut out = vec![];
            for nav_lang in window().navigator().languages() {
                let nav_lang = nav_lang.as_string().unwrap();
                if let Some(lang) = Lang::from_nav_lang(&nav_lang) {
                    if !out.contains(&lang) {
                        out.push(lang);
                    }
                }
            }
            break out;
        },
        pwa: {
            // Needs to match manifest
            let pwa = match window().match_media("(display-mode: standalone)") {
                Ok(v) => if let Some(v) = v {
                    v.matches()
                } else {
                    false
                },
                Err(e) => {
                    log.log_js("Error running media query to determine if PWA", &e);
                    false
                },
            };
            log.log(&format!("Detected pwa, activating (safari?) pwa workarounds: {}", pwa));
            pwa
        },
    }
}
