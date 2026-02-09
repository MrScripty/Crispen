#[cfg(feature = "ocio")]
use crispen_core::transform::params::ColorSpaceId;
#[cfg(feature = "ocio")]
use crispen_ocio::OcioConfig;

#[cfg(feature = "ocio")]
pub fn map_detected_to_ocio_name(detected_space: ColorSpaceId, config: &OcioConfig) -> String {
    let names = config.color_space_names();
    if names.is_empty() {
        return fallback_name(config);
    }

    for candidate in candidates_for_space(detected_space) {
        if let Some(found) = names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(candidate))
        {
            return found.clone();
        }
    }

    for candidate in candidates_for_space(detected_space) {
        let normalized_candidate = normalize(candidate);
        if let Some(found) = names.iter().find(|name| {
            let n = normalize(name);
            n.contains(&normalized_candidate) || normalized_candidate.contains(&n)
        }) {
            return found.clone();
        }
    }

    fallback_name(config)
}

#[cfg(feature = "ocio")]
fn fallback_name(config: &OcioConfig) -> String {
    let spaces = config.color_space_names();
    if let Some(found) = spaces
        .iter()
        .find(|name| name.eq_ignore_ascii_case("sRGB - Texture"))
    {
        return found.clone();
    }
    spaces
        .into_iter()
        .next()
        .unwrap_or_else(|| "sRGB - Texture".to_string())
}

#[cfg(feature = "ocio")]
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(feature = "ocio")]
fn candidates_for_space(space: ColorSpaceId) -> &'static [&'static str] {
    match space {
        ColorSpaceId::Srgb => &["sRGB - Texture", "sRGB"],
        ColorSpaceId::LinearSrgb => &["Linear Rec.709 (sRGB)", "lin srgb", "linear srgb"],
        ColorSpaceId::AcesCg => &["ACEScg"],
        ColorSpaceId::Aces2065_1 => &["ACES2065-1"],
        ColorSpaceId::ArriLogC3 => &["ARRI LogC3 (EI800)", "ARRI LogC3"],
        ColorSpaceId::ArriLogC4 => &["ARRI LogC4"],
        ColorSpaceId::SLog3 => &["Sony S-Log3 S-Gamut3.Cine", "S-Log3"],
        ColorSpaceId::RedLog3G10 => &["RED Log3G10 REDWideGamutRGB", "RED Log3G10"],
        ColorSpaceId::VLog => &["Panasonic V-Log V-Gamut", "V-Log"],
        ColorSpaceId::Rec2020 => &["Rec.2020", "Rec2020"],
        ColorSpaceId::DciP3 => &["Display P3 - Display", "Display P3", "DCI-P3"],
        ColorSpaceId::AcesCc => &["ACEScc"],
        ColorSpaceId::AcesCct => &["ACEScct"],
        ColorSpaceId::Custom(_) => &["sRGB - Texture"],
    }
}
