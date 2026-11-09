/** The languages each speech model understands, for the list in Settings. */

/** Parakeet V3 was trained on 25 European languages. */
export const PARAKEET_LANGUAGES = [
  "Bulgarian", "Croatian", "Czech", "Danish", "Dutch", "English", "Estonian", "Finnish", "French", "German",
  "Greek", "Hungarian", "Italian", "Latvian", "Lithuanian", "Maltese", "Polish", "Portuguese", "Romanian",
  "Russian", "Slovak", "Slovenian", "Spanish", "Swedish", "Ukrainian",
];

/** Whisper Small knows 99 languages. Whisper Turbo adds Cantonese. */
export const WHISPER_LANGUAGES = [
  "Afrikaans", "Albanian", "Amharic", "Arabic", "Armenian", "Assamese", "Azerbaijani", "Bashkir", "Basque",
  "Belarusian", "Bengali", "Bosnian", "Breton", "Bulgarian", "Burmese", "Catalan", "Chinese", "Croatian",
  "Czech", "Danish", "Dutch", "English", "Estonian", "Faroese", "Finnish", "French", "Galician", "Georgian",
  "German", "Greek", "Gujarati", "Haitian Creole", "Hausa", "Hawaiian", "Hebrew", "Hindi", "Hungarian",
  "Icelandic", "Indonesian", "Italian", "Japanese", "Javanese", "Kannada", "Kazakh", "Khmer", "Korean", "Lao",
  "Latin", "Latvian", "Lingala", "Lithuanian", "Luxembourgish", "Macedonian", "Malagasy", "Malay", "Malayalam",
  "Maltese", "Māori", "Marathi", "Mongolian", "Nepali", "Norwegian", "Norwegian Nynorsk", "Occitan", "Pashto",
  "Persian", "Polish", "Portuguese", "Punjabi", "Romanian", "Russian", "Sanskrit", "Serbian", "Shona", "Sindhi",
  "Sinhala", "Slovak", "Slovenian", "Somali", "Spanish", "Sundanese", "Swahili", "Swedish", "Tagalog", "Tajik",
  "Tamil", "Tatar", "Telugu", "Thai", "Tibetan", "Turkish", "Turkmen", "Ukrainian", "Urdu", "Uzbek",
  "Vietnamese", "Welsh", "Yiddish", "Yoruba",
];

export const MODEL_LANGUAGES: { id: string; name: string; summary: string; languages: string[] }[] = [
  {
    id: "parakeet",
    name: "Parakeet V3",
    summary: "25 European languages. No Japanese, Chinese or Korean.",
    languages: PARAKEET_LANGUAGES,
  },
  {
    id: "whisper-turbo",
    name: "Whisper Turbo",
    summary: "100 languages, Japanese, Chinese and Korean included.",
    languages: [...WHISPER_LANGUAGES, "Cantonese"].sort((a, b) => a.localeCompare(b)),
  },
  {
    id: "whisper-small",
    name: "Whisper Small",
    summary: "99 languages, the same as Turbo without Cantonese.",
    languages: WHISPER_LANGUAGES,
  },
];
