pub const STATIC_SAMPLER_GLOBAL_PREFIX: &str = "__air_sampler_state";

pub fn is_static_sampler_global(name: &str) -> bool {
    name.trim_start_matches('@')
        .starts_with(STATIC_SAMPLER_GLOBAL_PREFIX)
}

pub fn static_sampler_name_order(name: &str) -> (Vec<u64>, String) {
    let name = name.trim_start_matches('@');
    let components = name
        .strip_prefix(STATIC_SAMPLER_GLOBAL_PREFIX)
        .unwrap_or("")
        .split('.')
        .filter(|component| !component.is_empty())
        .map(|component| component.parse::<u64>().unwrap_or(u64::MAX))
        .collect();
    (components, name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_uniqued_sampler_names_do_not_share_an_order_key() {
        let names = [
            "@__air_sampler_state",
            "@__air_sampler_state.118.9",
            "__air_sampler_state.119",
            "@__air_sampler_state.7",
        ];
        let mut sorted = names.to_vec();
        sorted.sort_by_key(|name| static_sampler_name_order(name));
        assert_eq!(
            sorted,
            [
                "@__air_sampler_state",
                "@__air_sampler_state.7",
                "@__air_sampler_state.118.9",
                "__air_sampler_state.119",
            ],
            "the whole dot-separated suffix orders these, and the leading `@` is not part of it"
        );
        let keys = names
            .iter()
            .map(|name| static_sampler_name_order(name))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            keys.len(),
            names.len(),
            "no two distinct sampler globals may share a key, or the order of the tied pair is \
             decided by whichever scan collected them"
        );
    }

    #[test]
    fn only_the_air_static_sampler_global_is_recognised() {
        assert!(is_static_sampler_global("@__air_sampler_state.118"));
        assert!(is_static_sampler_global("__air_sampler_state"));
        assert!(!is_static_sampler_global("@__air_sampler"));
        assert!(!is_static_sampler_global("@sampler_state"));
    }
}
