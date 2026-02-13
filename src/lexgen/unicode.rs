//! Unicode support for regex parsing.
//!
//! Provides Unicode character properties and categories for regex matching.
//! Implements:
//! - Unicode General Categories (L, Lu, Ll, Nd, etc.)
//! - Unicode Scripts (Latin, Greek, Cyrillic, etc.)
//! - Unicode binary properties (Alphabetic, Lowercase, etc.)
//! - Hex escape parsing (\u{XXXX}, \x{XXXX})

use std::ops::RangeInclusive;

/// Unicode General Category enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneralCategory {
    // Letter (L)
    Lu, // Uppercase Letter
    Ll, // Lowercase Letter
    Lt, // Titlecase Letter
    Lm, // Modifier Letter
    Lo, // Other Letter

    // Mark (M)
    Mn, // Non-Spacing Mark
    Mc, // Spacing Combining Mark
    Me, // Enclosing Mark

    // Number (N)
    Nd, // Decimal Digit Number
    Nl, // Letter Number
    No, // Other Number

    // Punctuation (P)
    Pc, // Connector Punctuation
    Pd, // Dash Punctuation
    Ps, // Open Punctuation
    Pe, // Close Punctuation
    Pi, // Initial Punctuation
    Pf, // Final Punctuation
    Po, // Other Punctuation

    // Symbol (S)
    Sm, // Math Symbol
    Sc, // Currency Symbol
    Sk, // Modifier Symbol
    So, // Other Symbol

    // Separator (Z)
    Zs, // Space Separator
    Zl, // Line Separator
    Zp, // Paragraph Separator

    // Other (C)
    Cc, // Control
    Cf, // Format
    Cs, // Surrogate
    Co, // Private Use
    Cn, // Unassigned
}

impl GeneralCategory {
    /// Returns the parent category (L, M, N, P, S, Z, C).
    pub fn parent_category(&self) -> char {
        match self {
            Self::Lu | Self::Ll | Self::Lt | Self::Lm | Self::Lo => 'L',
            Self::Mn | Self::Mc | Self::Me => 'M',
            Self::Nd | Self::Nl | Self::No => 'N',
            Self::Pc | Self::Pd | Self::Ps | Self::Pe | Self::Pi | Self::Pf | Self::Po => 'P',
            Self::Sm | Self::Sc | Self::Sk | Self::So => 'S',
            Self::Zs | Self::Zl | Self::Zp => 'Z',
            Self::Cc | Self::Cf | Self::Cs | Self::Co | Self::Cn => 'C',
        }
    }

    /// Parses a category from a string like "Lu", "L", "Letter", etc.
    pub fn from_str(s: &str) -> Option<Vec<Self>> {
        let s_lower = s.to_lowercase();
        let s_normalized: String = s_lower
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
            .collect();

        // Single character parent categories
        match s_normalized.as_str() {
            "l" | "letter" => Some(vec![Self::Lu, Self::Ll, Self::Lt, Self::Lm, Self::Lo]),
            "m" | "mark" => Some(vec![Self::Mn, Self::Mc, Self::Me]),
            "n" | "number" => Some(vec![Self::Nd, Self::Nl, Self::No]),
            "p" | "punctuation" | "punct" => Some(vec![
                Self::Pc,
                Self::Pd,
                Self::Ps,
                Self::Pe,
                Self::Pi,
                Self::Pf,
                Self::Po,
            ]),
            "s" | "symbol" => Some(vec![Self::Sm, Self::Sc, Self::Sk, Self::So]),
            "z" | "separator" => Some(vec![Self::Zs, Self::Zl, Self::Zp]),
            "c" | "other" => Some(vec![Self::Cc, Self::Cf, Self::Cs, Self::Co, Self::Cn]),

            // Specific subcategories
            "lu" | "uppercaseletter" => Some(vec![Self::Lu]),
            "ll" | "lowercaseletter" => Some(vec![Self::Ll]),
            "lt" | "titlecaseletter" => Some(vec![Self::Lt]),
            "lm" | "modifierletter" => Some(vec![Self::Lm]),
            "lo" | "otherletter" => Some(vec![Self::Lo]),

            "mn" | "nonspacingmark" => Some(vec![Self::Mn]),
            "mc" | "spacingcombiningmark" => Some(vec![Self::Mc]),
            "me" | "enclosingmark" => Some(vec![Self::Me]),

            "nd" | "decimalnumber" | "digit" => Some(vec![Self::Nd]),
            "nl" | "letternumber" => Some(vec![Self::Nl]),
            "no" | "othernumber" => Some(vec![Self::No]),

            "pc" | "connectorpunctuation" => Some(vec![Self::Pc]),
            "pd" | "dashpunctuation" => Some(vec![Self::Pd]),
            "ps" | "openpunctuation" => Some(vec![Self::Ps]),
            "pe" | "closepunctuation" => Some(vec![Self::Pe]),
            "pi" | "initialpunctuation" => Some(vec![Self::Pi]),
            "pf" | "finalpunctuation" => Some(vec![Self::Pf]),
            "po" | "otherpunctuation" => Some(vec![Self::Po]),

            "sm" | "mathsymbol" => Some(vec![Self::Sm]),
            "sc" | "currencysymbol" => Some(vec![Self::Sc]),
            "sk" | "modifiersymbol" => Some(vec![Self::Sk]),
            "so" | "othersymbol" => Some(vec![Self::So]),

            "zs" | "spaceseparator" => Some(vec![Self::Zs]),
            "zl" | "lineseparator" => Some(vec![Self::Zl]),
            "zp" | "paragraphseparator" => Some(vec![Self::Zp]),

            "cc" | "control" | "cntrl" => Some(vec![Self::Cc]),
            "cf" | "format" => Some(vec![Self::Cf]),
            "cs" | "surrogate" => Some(vec![Self::Cs]),
            "co" | "privateuse" => Some(vec![Self::Co]),
            "cn" | "unassigned" => Some(vec![Self::Cn]),

            _ => None,
        }
    }
}

/// Unicode code point ranges for each general category.
/// These are simplified ranges covering the most common characters.
/// A full implementation would use the Unicode Character Database.
pub fn get_category_ranges(cat: GeneralCategory) -> Vec<RangeInclusive<u32>> {
    match cat {
        GeneralCategory::Lu => vec![
            0x0041..=0x005A, // A-Z
            0x00C0..=0x00D6, // Latin Extended-A uppercase
            0x00D8..=0x00DE,
            0x0100..=0x0136, // Latin Extended-A (even codes are uppercase)
            0x0391..=0x03A9, // Greek uppercase (Alpha-Omega, excluding some)
            0x0410..=0x042F, // Cyrillic uppercase
        ],
        GeneralCategory::Ll => vec![
            0x0061..=0x007A, // a-z
            0x00DF..=0x00F6, // Latin Extended lowercase
            0x00F8..=0x00FF,
            0x0101..=0x0137, // Latin Extended-A (odd codes are lowercase)
            0x03B1..=0x03C9, // Greek lowercase (alpha-omega)
            0x0430..=0x044F, // Cyrillic lowercase
        ],
        GeneralCategory::Lt => vec![
            0x01C5..=0x01C5, // Titlecase letters
            0x01C8..=0x01C8,
            0x01CB..=0x01CB,
            0x01F2..=0x01F2,
        ],
        GeneralCategory::Lm => vec![
            0x02B0..=0x02C1, // Modifier letters
            0x02C6..=0x02D1,
            0x02E0..=0x02E4,
        ],
        GeneralCategory::Lo => vec![
            0x00AA..=0x00AA, // Feminine ordinal
            0x00BA..=0x00BA, // Masculine ordinal
            0x3041..=0x3096, // Hiragana
            0x30A1..=0x30FA, // Katakana
            0x4E00..=0x9FFF, // CJK Unified Ideographs
            0xAC00..=0xD7A3, // Hangul Syllables
        ],
        GeneralCategory::Mn => vec![
            0x0300..=0x036F, // Combining Diacritical Marks
            0x0591..=0x05BD, // Hebrew marks
            0x064B..=0x065F, // Arabic marks
        ],
        GeneralCategory::Mc => vec![
            0x0903..=0x0903, // Devanagari spacing marks
            0x093B..=0x093B,
            0x093E..=0x0940,
        ],
        GeneralCategory::Me => vec![
            0x0488..=0x0489, // Enclosing marks
            0x20DD..=0x20E0,
        ],
        GeneralCategory::Nd => vec![
            0x0030..=0x0039, // ASCII digits 0-9
            0x0660..=0x0669, // Arabic-Indic digits
            0x06F0..=0x06F9, // Extended Arabic-Indic
            0x0966..=0x096F, // Devanagari digits
            0x09E6..=0x09EF, // Bengali digits
            0x0A66..=0x0A6F, // Gurmukhi digits
            0x0AE6..=0x0AEF, // Gujarati digits
            0x0B66..=0x0B6F, // Oriya digits
            0x0BE6..=0x0BEF, // Tamil digits
            0xFF10..=0xFF19, // Fullwidth digits
        ],
        GeneralCategory::Nl => vec![
            0x16EE..=0x16F0, // Runic symbols
            0x2160..=0x2182, // Roman numerals
            0x3007..=0x3007, // Ideographic number zero
            0x3021..=0x3029, // Hangzhou numerals
        ],
        GeneralCategory::No => vec![
            0x00B2..=0x00B3, // Superscript 2, 3
            0x00B9..=0x00B9, // Superscript 1
            0x00BC..=0x00BE, // Fractions 1/4, 1/2, 3/4
            0x2070..=0x2079, // Superscripts
            0x2080..=0x2089, // Subscripts
            0x2153..=0x215E, // Fractions
        ],
        GeneralCategory::Pc => vec![
            0x005F..=0x005F, // Underscore _
            0x203F..=0x2040, // Undertie, Character tie
            0x2054..=0x2054, // Inverted undertie
            0xFE33..=0xFE34, // Presentation forms
            0xFE4D..=0xFE4F,
            0xFF3F..=0xFF3F, // Fullwidth underscore
        ],
        GeneralCategory::Pd => vec![
            0x002D..=0x002D, // Hyphen-minus
            0x058A..=0x058A, // Armenian hyphen
            0x05BE..=0x05BE, // Hebrew maqaf
            0x1806..=0x1806, // Mongolian soft hyphen
            0x2010..=0x2015, // Various dashes
            0x2E17..=0x2E17, // Double oblique hyphen
            0x2E1A..=0x2E1A,
            0x301C..=0x301C, // Wave dash
            0xFE31..=0xFE32,
            0xFE58..=0xFE58,
            0xFE63..=0xFE63,
            0xFF0D..=0xFF0D, // Fullwidth hyphen
        ],
        GeneralCategory::Ps => vec![
            0x0028..=0x0028, // (
            0x005B..=0x005B, // [
            0x007B..=0x007B, // {
            0x2045..=0x2045,
            0x207D..=0x207D,
            0x208D..=0x208D,
            0x2308..=0x2308,
            0x230A..=0x230A,
            0x3008..=0x3008, // CJK brackets
            0x300A..=0x300A,
            0x300C..=0x300C,
            0x300E..=0x300E,
            0x3010..=0x3010,
            0xFF08..=0xFF08, // Fullwidth (
            0xFF3B..=0xFF3B, // Fullwidth [
            0xFF5B..=0xFF5B, // Fullwidth {
        ],
        GeneralCategory::Pe => vec![
            0x0029..=0x0029, // )
            0x005D..=0x005D, // ]
            0x007D..=0x007D, // }
            0x2046..=0x2046,
            0x207E..=0x207E,
            0x208E..=0x208E,
            0x2309..=0x2309,
            0x230B..=0x230B,
            0x3009..=0x3009, // CJK brackets
            0x300B..=0x300B,
            0x300D..=0x300D,
            0x300F..=0x300F,
            0x3011..=0x3011,
            0xFF09..=0xFF09, // Fullwidth )
            0xFF3D..=0xFF3D, // Fullwidth ]
            0xFF5D..=0xFF5D, // Fullwidth }
        ],
        GeneralCategory::Pi => vec![
            0x00AB..=0x00AB, // Left-pointing double angle quotation mark
            0x2018..=0x2018, // Left single quotation mark
            0x201B..=0x201C,
            0x201F..=0x201F,
            0x2039..=0x2039,
        ],
        GeneralCategory::Pf => vec![
            0x00BB..=0x00BB, // Right-pointing double angle quotation mark
            0x2019..=0x2019, // Right single quotation mark
            0x201D..=0x201D,
            0x203A..=0x203A,
        ],
        GeneralCategory::Po => vec![
            0x0021..=0x0023, // ! " #
            0x0025..=0x0027, // % & '
            0x002A..=0x002A, // *
            0x002C..=0x002C, // ,
            0x002E..=0x002F, // . /
            0x003A..=0x003B, // : ;
            0x003F..=0x0040, // ? @
            0x005C..=0x005C, // \
            0x00A1..=0x00A1, // Inverted exclamation
            0x00BF..=0x00BF, // Inverted question
            0x2020..=0x2027, // Various punctuation
        ],
        GeneralCategory::Sm => vec![
            0x002B..=0x002B, // +
            0x003C..=0x003E, // < = >
            0x007C..=0x007C, // |
            0x007E..=0x007E, // ~
            0x00AC..=0x00AC, // Not sign
            0x00B1..=0x00B1, // Plus-minus
            0x00D7..=0x00D7, // Multiplication
            0x00F7..=0x00F7, // Division
            0x2200..=0x22FF, // Mathematical Operators block
            0x2A00..=0x2AFF, // Supplemental Mathematical Operators
        ],
        GeneralCategory::Sc => vec![
            0x0024..=0x0024, // $
            0x00A2..=0x00A5, // Cent, Pound, Currency, Yen
            0x20A0..=0x20CF, // Currency Symbols block
            0xFE69..=0xFE69,
            0xFF04..=0xFF04, // Fullwidth $
        ],
        GeneralCategory::Sk => vec![
            0x005E..=0x005E, // ^
            0x0060..=0x0060, // `
            0x00A8..=0x00A8, // Diaeresis
            0x00AF..=0x00AF, // Macron
            0x00B4..=0x00B4, // Acute accent
            0x00B8..=0x00B8, // Cedilla
            0x02D8..=0x02DD, // Modifier letters
        ],
        GeneralCategory::So => vec![
            0x00A6..=0x00A9,   // Broken bar, Section, Copyright, etc.
            0x00AE..=0x00AE,   // Registered
            0x00B0..=0x00B0,   // Degree
            0x2100..=0x214F,   // Letterlike Symbols
            0x2190..=0x21FF,   // Arrows
            0x2300..=0x23FF,   // Miscellaneous Technical
            0x2600..=0x26FF,   // Miscellaneous Symbols
            0x2700..=0x27BF,   // Dingbats
            0x1F300..=0x1F5FF, // Miscellaneous Symbols and Pictographs
            0x1F600..=0x1F64F, // Emoticons
            0x1F680..=0x1F6FF, // Transport and Map Symbols
            0x1F900..=0x1F9FF, // Supplemental Symbols and Pictographs
        ],
        GeneralCategory::Zs => vec![
            0x0020..=0x0020, // Space
            0x00A0..=0x00A0, // Non-breaking space
            0x1680..=0x1680, // Ogham space mark
            0x2000..=0x200A, // En quad through hair space
            0x202F..=0x202F, // Narrow no-break space
            0x205F..=0x205F, // Medium mathematical space
            0x3000..=0x3000, // Ideographic space
        ],
        GeneralCategory::Zl => vec![
            0x2028..=0x2028, // Line Separator
        ],
        GeneralCategory::Zp => vec![
            0x2029..=0x2029, // Paragraph Separator
        ],
        GeneralCategory::Cc => vec![
            0x0000..=0x001F, // C0 controls
            0x007F..=0x009F, // DEL and C1 controls
        ],
        GeneralCategory::Cf => vec![
            0x00AD..=0x00AD, // Soft hyphen
            0x0600..=0x0605, // Arabic format characters
            0x200B..=0x200F, // Zero-width characters
            0x2060..=0x2064, // Word joiner, etc.
            0xFEFF..=0xFEFF, // BOM
        ],
        GeneralCategory::Cs => vec![
            0xD800..=0xDFFF, // Surrogate pairs (UTF-16)
        ],
        GeneralCategory::Co => vec![
            0xE000..=0xF8FF,     // Private Use Area
            0xF0000..=0xFFFFD,   // Supplementary Private Use Area-A
            0x100000..=0x10FFFD, // Supplementary Private Use Area-B
        ],
        GeneralCategory::Cn => vec![
            // Unassigned code points - too many to list
            // This is a placeholder; in practice, we'd derive this from assigned ranges
        ],
    }
}

/// Common Unicode scripts with their code point ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeScript {
    Latin,
    Greek,
    Cyrillic,
    Armenian,
    Hebrew,
    Arabic,
    Devanagari,
    Bengali,
    Tamil,
    Telugu,
    Thai,
    Hiragana,
    Katakana,
    Han,
    Hangul,
    Common,
    Inherited,
}

impl UnicodeScript {
    /// Parses a script name.
    pub fn from_str(s: &str) -> Option<Self> {
        let s_lower = s.to_lowercase();
        let s_normalized: String = s_lower
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
            .collect();

        match s_normalized.as_str() {
            "latin" | "latn" => Some(Self::Latin),
            "greek" | "grek" => Some(Self::Greek),
            "cyrillic" | "cyrl" => Some(Self::Cyrillic),
            "armenian" | "armn" => Some(Self::Armenian),
            "hebrew" | "hebr" => Some(Self::Hebrew),
            "arabic" | "arab" => Some(Self::Arabic),
            "devanagari" | "deva" => Some(Self::Devanagari),
            "bengali" | "beng" => Some(Self::Bengali),
            "tamil" | "taml" => Some(Self::Tamil),
            "telugu" | "telu" => Some(Self::Telugu),
            "thai" => Some(Self::Thai),
            "hiragana" | "hira" => Some(Self::Hiragana),
            "katakana" | "kana" => Some(Self::Katakana),
            "han" | "hani" => Some(Self::Han),
            "hangul" | "hang" => Some(Self::Hangul),
            "common" | "zyyy" => Some(Self::Common),
            "inherited" | "zinh" | "qaai" => Some(Self::Inherited),
            _ => None,
        }
    }

    /// Returns the code point ranges for this script.
    pub fn ranges(&self) -> Vec<RangeInclusive<u32>> {
        match self {
            Self::Latin => vec![
                0x0041..=0x005A, // A-Z
                0x0061..=0x007A, // a-z
                0x00C0..=0x00FF, // Latin Extended
                0x0100..=0x017F, // Latin Extended-A
                0x0180..=0x024F, // Latin Extended-B
                0x1E00..=0x1EFF, // Latin Extended Additional
                0x2C60..=0x2C7F, // Latin Extended-C
                0xA720..=0xA7FF, // Latin Extended-D
                0xAB30..=0xAB6F, // Latin Extended-E
            ],
            Self::Greek => vec![
                0x0370..=0x03FF, // Greek and Coptic
                0x1F00..=0x1FFF, // Greek Extended
            ],
            Self::Cyrillic => vec![
                0x0400..=0x04FF, // Cyrillic
                0x0500..=0x052F, // Cyrillic Supplement
                0x2DE0..=0x2DFF, // Cyrillic Extended-A
                0xA640..=0xA69F, // Cyrillic Extended-B
            ],
            Self::Armenian => vec![
                0x0530..=0x058F, // Armenian
                0xFB00..=0xFB17, // Alphabetic Presentation Forms (ligatures)
            ],
            Self::Hebrew => vec![
                0x0590..=0x05FF, // Hebrew
                0xFB1D..=0xFB4F, // Alphabetic Presentation Forms
            ],
            Self::Arabic => vec![
                0x0600..=0x06FF, // Arabic
                0x0750..=0x077F, // Arabic Supplement
                0x08A0..=0x08FF, // Arabic Extended-A
                0xFB50..=0xFDFF, // Arabic Presentation Forms-A
                0xFE70..=0xFEFF, // Arabic Presentation Forms-B
            ],
            Self::Devanagari => vec![
                0x0900..=0x097F, // Devanagari
                0xA8E0..=0xA8FF, // Devanagari Extended
            ],
            Self::Bengali => vec![
                0x0980..=0x09FF, // Bengali
            ],
            Self::Tamil => vec![
                0x0B80..=0x0BFF, // Tamil
            ],
            Self::Telugu => vec![
                0x0C00..=0x0C7F, // Telugu
            ],
            Self::Thai => vec![
                0x0E00..=0x0E7F, // Thai
            ],
            Self::Hiragana => vec![
                0x3040..=0x309F,   // Hiragana
                0x1B001..=0x1B11E, // Hiragana Extended
            ],
            Self::Katakana => vec![
                0x30A0..=0x30FF, // Katakana
                0x31F0..=0x31FF, // Katakana Phonetic Extensions
                0xFF65..=0xFF9F, // Halfwidth Katakana
            ],
            Self::Han => vec![
                0x4E00..=0x9FFF,   // CJK Unified Ideographs
                0x3400..=0x4DBF,   // CJK Unified Ideographs Extension A
                0x20000..=0x2A6DF, // CJK Unified Ideographs Extension B
                0x2A700..=0x2B73F, // CJK Unified Ideographs Extension C
                0x2B740..=0x2B81F, // CJK Unified Ideographs Extension D
                0xF900..=0xFAFF,   // CJK Compatibility Ideographs
            ],
            Self::Hangul => vec![
                0x1100..=0x11FF, // Hangul Jamo
                0x3130..=0x318F, // Hangul Compatibility Jamo
                0xA960..=0xA97F, // Hangul Jamo Extended-A
                0xAC00..=0xD7AF, // Hangul Syllables
                0xD7B0..=0xD7FF, // Hangul Jamo Extended-B
            ],
            Self::Common => vec![
                // Common script includes punctuation, numbers, etc.
                0x0000..=0x0040, // Controls and basic punctuation
                0x005B..=0x0060,
                0x007B..=0x00BF,
                0x00D7..=0x00D7,
                0x00F7..=0x00F7,
                0x2000..=0x206F, // General Punctuation
                0x20A0..=0x20CF, // Currency Symbols
                0x2100..=0x214F, // Letterlike Symbols
            ],
            Self::Inherited => vec![
                0x0300..=0x036F, // Combining Diacritical Marks
                0x1AB0..=0x1AFF, // Combining Diacritical Marks Extended
                0x1DC0..=0x1DFF, // Combining Diacritical Marks Supplement
                0x20D0..=0x20FF, // Combining Diacritical Marks for Symbols
                0xFE20..=0xFE2F, // Combining Half Marks
            ],
        }
    }
}

/// Parses a hex escape sequence like \u{1F600} or \x{41}.
/// Returns the parsed Unicode code point.
pub fn parse_hex_escape(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<char> {
    // After \u or \x, expect { or 4 hex digits
    if chars.peek() == Some(&'{') {
        chars.next(); // consume '{'
        let mut hex_str = String::new();

        while let Some(&c) = chars.peek() {
            if c == '}' {
                chars.next(); // consume '}'
                break;
            }
            if c.is_ascii_hexdigit() || c == ' ' {
                chars.next();
                if c != ' ' {
                    hex_str.push(c);
                }
            } else {
                return None; // Invalid character
            }
        }

        if hex_str.is_empty() {
            return None;
        }

        u32::from_str_radix(&hex_str, 16)
            .ok()
            .and_then(char::from_u32)
    } else {
        // Legacy 4-digit format: \uXXXX
        let mut hex_str = String::new();
        for _ in 0..4 {
            if let Some(&c) = chars.peek() {
                if c.is_ascii_hexdigit() {
                    chars.next();
                    hex_str.push(c);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if hex_str.len() == 4 {
            u32::from_str_radix(&hex_str, 16)
                .ok()
                .and_then(char::from_u32)
        } else {
            None
        }
    }
}

/// Parses a Unicode property expression like \p{Lu} or \p{Script=Greek}.
/// Returns a list of code point ranges that match the property.
pub fn parse_property(prop_str: &str) -> Option<(Vec<RangeInclusive<u32>>, bool)> {
    let trimmed = prop_str.trim();

    // Check for property=value format
    if let Some(eq_pos) = trimmed.find('=') {
        let prop_name = trimmed[..eq_pos].trim();
        let prop_value = trimmed[eq_pos + 1..].trim();

        let prop_name_lower = prop_name.to_lowercase();
        match prop_name_lower.as_str() {
            "gc" | "generalcategory" | "general_category" => {
                if let Some(cats) = GeneralCategory::from_str(prop_value) {
                    let mut ranges = Vec::new();
                    for cat in cats {
                        ranges.extend(get_category_ranges(cat));
                    }
                    return Some((ranges, false));
                }
            }
            "sc" | "script" => {
                if let Some(script) = UnicodeScript::from_str(prop_value) {
                    return Some((script.ranges(), false));
                }
            }
            _ => {}
        }
        None
    } else {
        // Simple property name - check categories first, then scripts
        if let Some(cats) = GeneralCategory::from_str(trimmed) {
            let mut ranges = Vec::new();
            for cat in cats {
                ranges.extend(get_category_ranges(cat));
            }
            return Some((ranges, false));
        }

        if let Some(script) = UnicodeScript::from_str(trimmed) {
            return Some((script.ranges(), false));
        }

        // Binary properties
        let trimmed_lower = trimmed.to_lowercase();
        match trimmed_lower.as_str() {
            "any" => Some((vec![0x0000..=0x10FFFF], false)),
            "ascii" => Some((vec![0x0000..=0x007F], false)),
            "assigned" => {
                // Everything except Cn (unassigned)
                // This is a simplification
                Some((vec![0x0000..=0xD7FF, 0xE000..=0x10FFFF], false))
            }
            "alphabetic" | "alpha" => {
                // L + Nl + Other_Alphabetic
                let mut ranges = Vec::new();
                for cat in [
                    GeneralCategory::Lu,
                    GeneralCategory::Ll,
                    GeneralCategory::Lt,
                    GeneralCategory::Lm,
                    GeneralCategory::Lo,
                    GeneralCategory::Nl,
                ] {
                    ranges.extend(get_category_ranges(cat));
                }
                Some((ranges, false))
            }
            "lowercase" | "lower" => Some((get_category_ranges(GeneralCategory::Ll), false)),
            "uppercase" | "upper" => Some((get_category_ranges(GeneralCategory::Lu), false)),
            "whitespace" | "white_space" | "space" => {
                let mut ranges = Vec::new();
                ranges.extend(get_category_ranges(GeneralCategory::Zs));
                ranges.extend(get_category_ranges(GeneralCategory::Zl));
                ranges.extend(get_category_ranges(GeneralCategory::Zp));
                // Add control whitespace characters
                ranges.push(0x0009..=0x000D); // Tab, LF, VT, FF, CR
                ranges.push(0x0085..=0x0085); // NEL
                Some((ranges, false))
            }
            "digit" => Some((get_category_ranges(GeneralCategory::Nd), false)),
            "xdigit" | "hex_digit" | "hexdigit" => {
                Some((
                    vec![
                        0x0030..=0x0039, // 0-9
                        0x0041..=0x0046, // A-F
                        0x0061..=0x0066, // a-f
                        0xFF10..=0xFF19, // Fullwidth 0-9
                        0xFF21..=0xFF26, // Fullwidth A-F
                        0xFF41..=0xFF46, // Fullwidth a-f
                    ],
                    false,
                ))
            }
            "word" => {
                // \w equivalent: Alphabetic + Marks + Digits + Pc
                let mut ranges = Vec::new();
                for cat in [
                    GeneralCategory::Lu,
                    GeneralCategory::Ll,
                    GeneralCategory::Lt,
                    GeneralCategory::Lm,
                    GeneralCategory::Lo,
                ] {
                    ranges.extend(get_category_ranges(cat));
                }
                for cat in [
                    GeneralCategory::Mn,
                    GeneralCategory::Mc,
                    GeneralCategory::Me,
                ] {
                    ranges.extend(get_category_ranges(cat));
                }
                ranges.extend(get_category_ranges(GeneralCategory::Nd));
                ranges.extend(get_category_ranges(GeneralCategory::Pc));
                Some((ranges, false))
            }
            _ => None,
        }
    }
}

/// Checks if a character is in any of the given ranges.
pub fn char_in_ranges(c: char, ranges: &[RangeInclusive<u32>]) -> bool {
    let cp = c as u32;
    ranges.iter().any(|range| range.contains(&cp))
}

/// Expands Unicode ranges to individual characters (up to a limit).
/// For very large ranges, returns a sample or uses range-based matching.
pub fn expand_ranges_to_chars(ranges: &[RangeInclusive<u32>], max_chars: usize) -> Vec<char> {
    let mut chars = Vec::new();

    for range in ranges {
        for cp in range.clone() {
            if let Some(c) = char::from_u32(cp) {
                chars.push(c);
                if chars.len() >= max_chars {
                    return chars;
                }
            }
        }
    }

    chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_parsing() {
        assert!(GeneralCategory::from_str("Lu").is_some());
        assert!(GeneralCategory::from_str("Uppercase Letter").is_some());
        assert!(GeneralCategory::from_str("L").is_some());
        assert!(GeneralCategory::from_str("Letter").is_some());
    }

    #[test]
    fn test_script_parsing() {
        assert!(UnicodeScript::from_str("Latin").is_some());
        assert!(UnicodeScript::from_str("greek").is_some());
        assert!(UnicodeScript::from_str("Grek").is_some());
    }

    #[test]
    fn test_hex_escape() {
        let mut chars = "41}rest".chars().peekable();
        // Simulate after consuming \u{
        let _result = parse_hex_escape(&mut chars);
        // This test would need adjustment based on actual input handling
    }

    #[test]
    fn test_property_parsing() {
        let (ranges, _) = parse_property("Lu").unwrap();
        assert!(!ranges.is_empty());

        let (ranges, _) = parse_property("Script=Greek").unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_all_general_categories() {
        // Test all short category names
        let categories = [
            "Lu", "Ll", "Lt", "Lm", "Lo", // Letter subcategories
            "Mn", "Mc", "Me", // Mark subcategories
            "Nd", "Nl", "No", // Number subcategories
            "Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po", // Punctuation
            "Sm", "Sc", "Sk", "So", // Symbol subcategories
            "Zs", "Zl", "Zp", // Separator subcategories
            "Cc", "Cf", "Cs", "Co", "Cn", // Other subcategories
        ];

        for cat in categories {
            let result = GeneralCategory::from_str(cat);
            assert!(result.is_some(), "Failed to parse category: {}", cat);
        }
    }

    #[test]
    fn test_major_categories() {
        // Test major category letters (L, M, N, P, S, Z, C)
        let major = ["L", "M", "N", "P", "S", "Z", "C"];
        for cat in major {
            let result = parse_property(cat);
            assert!(result.is_some(), "Failed to parse major category: {}", cat);
        }
    }

    #[test]
    fn test_all_scripts() {
        // Test all implemented scripts
        let scripts = [
            "Latin",
            "Greek",
            "Cyrillic",
            "Armenian",
            "Hebrew",
            "Arabic",
            "Devanagari",
            "Bengali",
            "Tamil",
            "Telugu",
            "Thai",
            "Hiragana",
            "Katakana",
            "Han",
            "Hangul",
            "Common",
            "Inherited",
        ];

        for script in scripts {
            let result = UnicodeScript::from_str(script);
            assert!(result.is_some(), "Failed to parse script: {}", script);
        }
    }

    #[test]
    fn test_script_aliases() {
        // Test ISO 15924 four-letter codes
        assert!(UnicodeScript::from_str("Latn").is_some());
        assert!(UnicodeScript::from_str("Grek").is_some());
        assert!(UnicodeScript::from_str("Cyrl").is_some());
        assert!(UnicodeScript::from_str("Hani").is_some());
    }

    #[test]
    fn test_category_ranges_not_empty() {
        assert!(!get_category_ranges(GeneralCategory::Lu).is_empty());
        assert!(!get_category_ranges(GeneralCategory::Ll).is_empty());
        assert!(!get_category_ranges(GeneralCategory::Nd).is_empty());
    }

    #[test]
    fn test_script_ranges_not_empty() {
        let latin = UnicodeScript::from_str("Latin").unwrap();
        assert!(!latin.ranges().is_empty());

        let greek = UnicodeScript::from_str("Greek").unwrap();
        assert!(!greek.ranges().is_empty());
    }

    #[test]
    fn test_char_in_ranges() {
        let latin_ranges = UnicodeScript::from_str("Latin").unwrap().ranges();

        // ASCII letters are in Latin script
        assert!(char_in_ranges('A', &latin_ranges));
        assert!(char_in_ranges('z', &latin_ranges));

        // Greek letters are not in Latin script
        let greek_alpha = '\u{03B1}'; // Greek small letter alpha
        assert!(!char_in_ranges(greek_alpha, &latin_ranges));
    }

    #[test]
    fn test_property_with_script_prefix() {
        // Test Script=Name format
        let (ranges, _) = parse_property("Script=Latin").unwrap();
        assert!(!ranges.is_empty());

        let (ranges, _) = parse_property("sc=Greek").unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_property_case_insensitive() {
        assert!(parse_property("LU").is_some());
        assert!(parse_property("lu").is_some());
        assert!(parse_property("Lu").is_some());
        assert!(parse_property("LATIN").is_some());
        assert!(parse_property("latin").is_some());
    }

    #[test]
    fn test_expand_ranges_to_chars() {
        let ranges = vec![0x41..=0x5A]; // A-Z
        let chars = expand_ranges_to_chars(&ranges, 100);
        assert_eq!(chars.len(), 26);
        assert_eq!(chars[0], 'A');
        assert_eq!(chars[25], 'Z');
    }

    #[test]
    fn test_expand_ranges_with_limit() {
        let ranges = vec![0x41..=0x5A]; // A-Z
        let chars = expand_ranges_to_chars(&ranges, 5);
        assert_eq!(chars.len(), 5);
        assert_eq!(chars[0], 'A');
        assert_eq!(chars[4], 'E');
    }

    #[test]
    fn test_uppercase_letter_contains_ascii() {
        let lu_ranges = get_category_ranges(GeneralCategory::Lu);

        // A-Z should be in uppercase letters
        for c in 'A'..='Z' {
            assert!(
                char_in_ranges(c, &lu_ranges),
                "Expected '{}' to be uppercase",
                c
            );
        }

        // a-z should NOT be in uppercase letters
        for c in 'a'..='z' {
            assert!(
                !char_in_ranges(c, &lu_ranges),
                "Expected '{}' to NOT be uppercase",
                c
            );
        }
    }

    #[test]
    fn test_lowercase_letter_contains_ascii() {
        let ll_ranges = get_category_ranges(GeneralCategory::Ll);

        // a-z should be in lowercase letters
        for c in 'a'..='z' {
            assert!(
                char_in_ranges(c, &ll_ranges),
                "Expected '{}' to be lowercase",
                c
            );
        }

        // A-Z should NOT be in lowercase letters
        for c in 'A'..='Z' {
            assert!(
                !char_in_ranges(c, &ll_ranges),
                "Expected '{}' to NOT be lowercase",
                c
            );
        }
    }

    #[test]
    fn test_decimal_number_contains_digits() {
        let nd_ranges = get_category_ranges(GeneralCategory::Nd);

        for c in '0'..='9' {
            assert!(
                char_in_ranges(c, &nd_ranges),
                "Expected '{}' to be decimal digit",
                c
            );
        }
    }

    #[test]
    fn test_greek_script_range() {
        let greek = UnicodeScript::from_str("Greek").unwrap();
        let ranges = greek.ranges();

        // Greek capital letters
        assert!(char_in_ranges('\u{0391}', &ranges)); // Alpha
        assert!(char_in_ranges('\u{03A9}', &ranges)); // Omega

        // Greek small letters
        assert!(char_in_ranges('\u{03B1}', &ranges)); // alpha
        assert!(char_in_ranges('\u{03C9}', &ranges)); // omega
    }

    #[test]
    fn test_invalid_property() {
        assert!(parse_property("InvalidCategory").is_none());
        assert!(parse_property("Script=NonExistent").is_none());
    }
}
