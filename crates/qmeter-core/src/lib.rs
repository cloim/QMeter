pub fn crate_name() -> &'static str {
    "qmeter-core"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "qmeter-core");
    }
}

