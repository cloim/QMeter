pub mod snapshot;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(env!("CARGO_PKG_NAME"), "qmeter-core");
    }
}
