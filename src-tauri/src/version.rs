use std::sync::LazyLock;

/// Versión de compilación en formato SemVer a partir de `git describe`:
/// `1.2.0`, `1.2.0+3.g4f8a9b` o, con árbol sucio, `...+3.g4f8a9b.20261030193600`.
pub static VERSION: LazyLock<String> = LazyLock::new(|| {
    compose_version(
        env!("VERGEN_GIT_DESCRIBE"),
        env!("VERGEN_GIT_DIRTY") == "true",
        env!("VERGEN_BUILD_TIMESTAMP"),
    )
});

fn compose_version(raw_describe: &str, is_dirty: bool, iso_time: &str) -> String {
    let clean_desc = raw_describe.strip_suffix("-dirty").unwrap_or(raw_describe);

    // TAG-N-gHASH -> TAG+N.gHASH
    let base_semver = match clean_desc.rsplit_once("-g") {
        Some((tag_and_count, hash)) => match tag_and_count.rsplit_once('-') {
            Some((tag, count)) => format!("{tag}+{count}.g{hash}"),
            None => clean_desc.to_string(),
        },
        None => clean_desc.to_string(),
    };

    if !is_dirty {
        return base_semver;
    }
    // "2026-10-30T19:36:00..." -> "20261030193600"
    let compact_ts: String = iso_time.chars().take(19).filter(char::is_ascii_digit).collect();
    let sep = if base_semver.contains('+') { "." } else { "+" };
    format!("{base_semver}{sep}{compact_ts}")
}

#[cfg(test)]
mod tests {
    use super::compose_version;

    #[test]
    fn composes_semver() {
        let ts = "2026-10-30T19:36:00.000000000Z";
        assert_eq!(compose_version("1.2.0", false, ts), "1.2.0");
        assert_eq!(compose_version("1.2.0-3-g4f8a9b", false, ts), "1.2.0+3.g4f8a9b");
        assert_eq!(compose_version("1.2.0-dirty", true, ts), "1.2.0+20261030193600");
        assert_eq!(
            compose_version("1.0.0-alpha.2-4-g09ab330-dirty", true, ts),
            "1.0.0-alpha.2+4.g09ab330.20261030193600"
        );
    }
}
