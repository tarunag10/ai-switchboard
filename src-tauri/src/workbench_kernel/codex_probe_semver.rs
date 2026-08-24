//! Minimal strict SemVer syntax validation for content-free probe metadata.

pub(super) fn is_strict_semver(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let (without_build, build) = split_once(value, '+');
    if build.is_some_and(|part| !valid_identifiers(part, false)) {
        return false;
    }
    let (core, prerelease) = split_once(without_build, '-');
    if prerelease.is_some_and(|part| !valid_identifiers(part, true)) {
        return false;
    }
    let components = core.split('.').collect::<Vec<_>>();
    components.len() == 3 && components.into_iter().all(valid_core_number)
}

fn split_once(value: &str, separator: char) -> (&str, Option<&str>) {
    match value.split_once(separator) {
        Some((left, right)) => (left, Some(right)),
        None => (value, None),
    }
}

fn valid_core_number(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn valid_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!reject_numeric_leading_zero
                    || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}
